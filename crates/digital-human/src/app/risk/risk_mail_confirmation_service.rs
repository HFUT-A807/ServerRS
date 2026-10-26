use crate::domain::risk::risk_mail_notification::RiskMailLinkT;
use crate::{domain::risk::risk_mail_notification::RiskMailRepoT, shared::error::AppError};
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct RiskMailConfirmationService {
    repo: Arc<dyn RiskMailRepoT>,
    secret: String,
    public_base_url: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    purpose: String,
}

impl RiskMailConfirmationService {
    pub fn new(repo: Arc<dyn RiskMailRepoT>, secret: String, public_base_url: String) -> Self {
        Self {
            repo,
            secret,
            public_base_url,
        }
    }

    pub async fn confirm(&self, token: &str) -> Result<bool, AppError> {
        if token.len() > 2048 {
            return Ok(false);
        }
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 0;
        let Ok(data) = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        ) else {
            return Ok(false);
        };
        if data.claims.purpose != "guardian-risk-mail-confirmation"
            || uuid::Uuid::parse_str(&data.claims.sub).is_err()
        {
            return Ok(false);
        }
        self.repo.acknowledge(&data.claims.sub).await
    }
}

impl RiskMailLinkT for RiskMailConfirmationService {
    fn confirmation_url(&self, id: &str, created_at: DateTime<Utc>) -> Result<String, AppError> {
        let token = encode(
            &Header::new(Algorithm::HS256),
            &Claims {
                sub: id.to_string(),
                exp: (created_at + chrono::Duration::days(7)).timestamp() as usize,
                purpose: "guardian-risk-mail-confirmation".into(),
            },
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|_| AppError::internal("could not create mail confirmation"))?;
        // Fragment is not sent in HTTP requests or server access logs.
        Ok(format!(
            "{}/mail/confirm#{token}",
            self.public_base_url.trim_end_matches('/')
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::risk::risk_mail_notification::RiskMailNotification;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Repo(AtomicUsize);
    #[async_trait]
    impl RiskMailRepoT for Repo {
        async fn enqueue(&self, _: &RiskMailNotification) -> Result<bool, AppError> {
            unreachable!()
        }
        async fn due(&self) -> Result<Vec<RiskMailNotification>, AppError> {
            unreachable!()
        }
        async fn claim(&self, _: &str) -> Result<bool, AppError> {
            unreachable!()
        }
        async fn sent(&self, _: &str, _: bool) -> Result<(), AppError> {
            unreachable!()
        }
        async fn cancel(&self, _: &str) -> Result<(), AppError> {
            unreachable!()
        }
        async fn acknowledge(&self, _: &str) -> Result<bool, AppError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(true)
        }
    }
    #[tokio::test]
    async fn risk_mail_confirmation_accepts_only_valid_purpose_signature_and_expiry() {
        let repo = Arc::new(Repo(AtomicUsize::new(0)));
        let service = RiskMailConfirmationService::new(
            repo.clone(),
            "test-secret-with-at-least-32-characters".into(),
            "https://example.test".into(),
        );
        let id = uuid::Uuid::new_v4().to_string();
        let url = service.confirmation_url(&id, Utc::now()).unwrap();
        assert!(url.starts_with("https://example.test/mail/confirm#"));
        let token = url.split('#').nth(1).unwrap();
        assert!(service.confirm(token).await.unwrap());
        assert!(service.confirm(token).await.unwrap());
        let expired = service
            .confirmation_url(&id, Utc::now() - chrono::Duration::days(8))
            .unwrap();
        assert!(
            !service
                .confirm(expired.split('#').nth(1).unwrap())
                .await
                .unwrap()
        );
        assert!(!service.confirm("invalid").await.unwrap());
        let mut altered = token.to_string();
        altered.push('x');
        assert!(!service.confirm(&altered).await.unwrap());
        let wrong = encode(
            &Header::new(Algorithm::HS256),
            &Claims {
                sub: id,
                exp: (Utc::now() + chrono::Duration::days(1)).timestamp() as usize,
                purpose: "access".into(),
            },
            &EncodingKey::from_secret(service.secret.as_bytes()),
        )
        .unwrap();
        assert!(!service.confirm(&wrong).await.unwrap());
        assert_eq!(repo.0.load(Ordering::SeqCst), 2);
    }
}
