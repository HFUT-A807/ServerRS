//! One-use P-256 / HKDF-SHA256 / AES-256-GCM login envelopes.
//! HTTPS is still required to authenticate the key exchange and protect tokens.
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use ring::{aead, agreement, hkdf, rand::SystemRandom};
use serde::{Deserialize, Serialize};

use crate::api::{dto::auth_dto::LoginRequest, error::ApiError};

const TTL: Duration = Duration::from_secs(120);
const MAX_PENDING: usize = 1024;
const INFO: &[u8] = b"serverrs-admin-login-v1";

struct PendingKey {
    key: agreement::EphemeralPrivateKey,
    created: Instant,
}

#[derive(Default)]
pub struct LoginCrypto {
    pending: Mutex<HashMap<String, PendingKey>>,
}

#[derive(Serialize)]
pub struct LoginChallenge {
    pub challenge_id: String,
    pub public_key: String,
    pub expires_in: u64,
}

// Deliberately no Debug: authentication payloads must never enter logs.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncryptedLogin {
    pub challenge_id: String,
    pub public_key: String,
    pub iv: String,
    pub ciphertext: String,
}

fn invalid() -> ApiError {
    ApiError::Validation("登录加密数据无效或已过期，请重新登录".into())
}

fn derive_key(secret: &[u8], id: &str) -> Result<aead::LessSafeKey, ring::error::Unspecified> {
    let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, id.as_bytes()).extract(secret);
    let info = [INFO];
    let okm = prk.expand(&info, &aead::AES_256_GCM)?;
    Ok(aead::LessSafeKey::new(aead::UnboundKey::from(okm)))
}

impl LoginCrypto {
    pub fn challenge(&self) -> Result<LoginChallenge, ApiError> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| ApiError::Internal("login key store unavailable".into()))?;
        pending.retain(|_, value| value.created.elapsed() < TTL);
        if pending.len() >= MAX_PENDING {
            return Err(ApiError::Forbidden("登录请求过多，请稍后重试".into()));
        }
        let key =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &SystemRandom::new())
                .map_err(|_| ApiError::Internal("login key generation failed".into()))?;
        let public_key = key
            .compute_public_key()
            .map_err(|_| ApiError::Internal("login key generation failed".into()))?;
        let challenge_id = uuid::Uuid::new_v4().to_string();
        let response = LoginChallenge {
            challenge_id: challenge_id.clone(),
            public_key: STANDARD.encode(public_key.as_ref()),
            expires_in: TTL.as_secs(),
        };
        pending.insert(
            challenge_id,
            PendingKey {
                key,
                created: Instant::now(),
            },
        );
        Ok(response)
    }

    pub fn decrypt(&self, envelope: EncryptedLogin) -> Result<LoginRequest, ApiError> {
        // Consume before parsing/decrypting, so even failed attempts cannot reuse a key.
        let pending = self
            .pending
            .lock()
            .map_err(|_| ApiError::Internal("login key store unavailable".into()))?
            .remove(&envelope.challenge_id)
            .ok_or_else(invalid)?;
        if pending.created.elapsed() >= TTL
            || envelope.public_key.len() > 88
            || envelope.iv.len() > 16
            || envelope.ciphertext.len() > 4096
        {
            return Err(invalid());
        }
        let public_key = STANDARD
            .decode(envelope.public_key)
            .map_err(|_| invalid())?;
        let iv: [u8; 12] = STANDARD
            .decode(envelope.iv)
            .map_err(|_| invalid())?
            .try_into()
            .map_err(|_| invalid())?;
        let mut ciphertext = STANDARD
            .decode(envelope.ciphertext)
            .map_err(|_| invalid())?;
        let peer = agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, public_key);
        let key = agreement::agree_ephemeral(pending.key, &peer, |secret| {
            derive_key(secret, &envelope.challenge_id)
        })
        .map_err(|_| invalid())?
        .map_err(|_| invalid())?;
        let plaintext = key
            .open_in_place(
                aead::Nonce::assume_unique_for_key(iv),
                aead::Aad::from(envelope.challenge_id.as_bytes()),
                &mut ciphertext,
            )
            .map_err(|_| invalid())?;
        // Do not include parser errors: they may contain credential fragments.
        serde_json::from_slice(plaintext).map_err(|_| invalid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(challenge: &LoginChallenge) -> EncryptedLogin {
        let private =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &SystemRandom::new())
                .unwrap();
        let public_key = STANDARD.encode(private.compute_public_key().unwrap().as_ref());
        let peer = agreement::UnparsedPublicKey::new(
            &agreement::ECDH_P256,
            STANDARD.decode(&challenge.public_key).unwrap(),
        );
        let key = agreement::agree_ephemeral(private, &peer, |secret| {
            derive_key(secret, &challenge.challenge_id)
        })
        .unwrap()
        .unwrap();
        let iv = [7; 12];
        let mut ciphertext =
            br#"{"username":"admin_test","password":"test-only-password","device_id":"test"}"#
                .to_vec();
        key.seal_in_place_append_tag(
            aead::Nonce::assume_unique_for_key(iv),
            aead::Aad::from(challenge.challenge_id.as_bytes()),
            &mut ciphertext,
        )
        .unwrap();
        EncryptedLogin {
            challenge_id: challenge.challenge_id.clone(),
            public_key,
            iv: STANDARD.encode(iv),
            ciphertext: STANDARD.encode(ciphertext),
        }
    }

    #[test]
    fn decrypts_credentials_once() {
        let crypto = LoginCrypto::default();
        let challenge = crypto.challenge().unwrap();
        let request = envelope(&challenge);
        let replay = envelope(&challenge);
        let payload = crypto.decrypt(request).unwrap();
        assert_eq!(payload.username, "admin_test");
        assert_eq!(payload.password, "test-only-password");
        assert_eq!(payload.device_id.as_deref(), Some("test"));
        assert!(crypto.decrypt(replay).is_err());
    }

    #[test]
    fn rejects_tampering_and_consumes_challenge() {
        let crypto = LoginCrypto::default();
        let challenge = crypto.challenge().unwrap();
        let mut request = envelope(&challenge);
        let mut bytes = STANDARD.decode(&request.ciphertext).unwrap();
        bytes[0] ^= 1;
        request.ciphertext = STANDARD.encode(bytes);
        assert!(crypto.decrypt(request).is_err());
        assert!(crypto.decrypt(envelope(&challenge)).is_err());
    }

    #[test]
    fn rejects_expired_keys_and_bounds_pending_keys() {
        let crypto = LoginCrypto::default();
        let challenge = crypto.challenge().unwrap();
        crypto
            .pending
            .lock()
            .unwrap()
            .get_mut(&challenge.challenge_id)
            .unwrap()
            .created = Instant::now() - TTL;
        assert!(crypto.decrypt(envelope(&challenge)).is_err());
        for _ in 0..MAX_PENDING {
            crypto.challenge().unwrap();
        }
        assert!(crypto.challenge().is_err());
        for pending in crypto.pending.lock().unwrap().values_mut() {
            pending.created = Instant::now() - TTL;
        }
        assert!(crypto.challenge().is_ok());
    }

    #[test]
    fn rejects_plaintext_and_malformed_envelopes() {
        assert!(
            serde_json::from_str::<EncryptedLogin>(r#"{"username":"admin","password":"secret"}"#)
                .is_err()
        );
        let crypto = LoginCrypto::default();
        let challenge = crypto.challenge().unwrap();
        let mut request = envelope(&challenge);
        request.public_key = "invalid".into();
        assert!(crypto.decrypt(request).is_err());
    }
}
