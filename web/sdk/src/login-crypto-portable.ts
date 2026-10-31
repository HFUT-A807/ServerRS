import { gcm } from '@noble/ciphers/aes.js'
import { p256 } from '@noble/curves/nist.js'
import { hkdf } from '@noble/hashes/hkdf.js'
import { sha256 } from '@noble/hashes/sha2.js'
import type { LoginChallenge } from './login-crypto.js'
import type { LoginRequest } from './types.js'

/** HTTP browsers expose getRandomValues but may not expose SubtleCrypto. */
export function encryptLoginPortable(
  payload: LoginRequest,
  challenge: LoginChallenge,
) {
  const encoder = new TextEncoder()
  const privateKey = p256.utils.randomSecretKey()
  let secret: Uint8Array | undefined
  let key: Uint8Array | undefined
  const plaintext = encoder.encode(JSON.stringify(payload))
  try {
    const serverKey = Uint8Array.from(atob(challenge.public_key), (char) =>
      char.charCodeAt(0),
    )
    // Web Crypto and ring use the x-coordinate; Noble returns a compressed point.
    secret = p256.getSharedSecret(privateKey, serverKey, true).slice(1)
    key = hkdf(
      sha256,
      secret,
      encoder.encode(challenge.challenge_id),
      encoder.encode('serverrs-admin-login-v1'),
      32,
    )
    const iv = globalThis.crypto.getRandomValues(new Uint8Array(12))
    const ciphertext = gcm(
      key,
      iv,
      encoder.encode(challenge.challenge_id),
    ).encrypt(plaintext)
    const base64 = (bytes: Uint8Array) => btoa(String.fromCharCode(...bytes))
    return {
      challenge_id: challenge.challenge_id,
      public_key: base64(p256.getPublicKey(privateKey, false)),
      iv: base64(iv),
      ciphertext: base64(ciphertext),
    }
  } finally {
    privateKey.fill(0)
    secret?.fill(0)
    key?.fill(0)
    plaintext.fill(0)
  }
}
