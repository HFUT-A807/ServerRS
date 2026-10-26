use super::detection_types::RiskLevel;
use crate::shared::error::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

pub const REMINDER_INTERVAL_SECS: i64 = 600;
pub const MAX_MAIL_ATTEMPTS: u8 = 3;

pub trait RiskMailLinkT: Send + Sync {
    fn confirmation_url(&self, id: &str, created_at: DateTime<Utc>) -> Result<String, AppError>;
}

#[derive(Debug, Clone)]
pub struct RiskMailNotification {
    pub id: String,
    pub user_id: u64,
    pub conversation_id: Option<u64>,
    pub recipient: String,
    pub display_name: String,
    pub risk_level: RiskLevel,
    pub attempts: u8,
    pub created_at: DateTime<Utc>,
    pub risk_audit_id: Option<u64>,
    pub is_test: bool,
}

#[async_trait]
pub trait RiskMailRepoT: Send + Sync {
    /// Returns false if an unacknowledged notification already exists for this user/level/recipient.
    async fn enqueue(&self, notification: &RiskMailNotification) -> Result<bool, AppError>;
    async fn due(&self) -> Result<Vec<RiskMailNotification>, AppError>;
    /// Atomically reserves one SMTP attempt. Also advances the deadline by ten minutes.
    async fn claim(&self, id: &str) -> Result<bool, AppError>;
    async fn sent(&self, id: &str, success: bool) -> Result<(), AppError>;
    async fn cancel(&self, id: &str) -> Result<(), AppError>;
    /// Idempotent; may acknowledge after the third attempt, but never an expired notification.
    async fn acknowledge(&self, id: &str) -> Result<bool, AppError>;
}
