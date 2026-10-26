use crate::{
    domain::risk::{
        detection_types::RiskLevel,
        risk_mail_notification::{RiskMailNotification, RiskMailRepoT},
    },
    shared::error::AppError,
};
use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};

pub struct RiskMailStore {
    db: DatabaseConnection,
}

impl RiskMailStore {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    pub async fn ensure_schema(&self) -> Result<(), AppError> {
        self.db
            .execute_raw(Statement::from_string(
                DbBackend::MySql,
                include_str!("risk_mail_schema.sql"),
            ))
            .await
            .map_err(|_| AppError::internal("mail notification schema unavailable"))?;
        Ok(())
    }
    async fn execute(&self, sql: &str, values: Vec<Value>) -> Result<u64, AppError> {
        self.db
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::MySql,
                sql,
                values,
            ))
            .await
            .map(|result| result.rows_affected())
            .map_err(|_| AppError::internal("mail notification persistence failed"))
    }
}

#[async_trait]
impl RiskMailRepoT for RiskMailStore {
    async fn enqueue(&self, n: &RiskMailNotification) -> Result<bool, AppError> {
        self.execute("UPDATE risk_mail_notifications SET status='expired' WHERE status='pending' AND expires_at <= UTC_TIMESTAMP(6)", vec![]).await?;
        let level = serde_json::to_string(&n.risk_level)
            .map_err(|_| AppError::internal("invalid mail level"))?;
        // ON DUPLICATE does not reset attempts or the original confirmation link.
        Ok(self.execute("INSERT INTO risk_mail_notifications (notification_id,user_id,conversation_id,recipient,display_name,risk_level,risk_audit_id,is_test,next_send_at,created_at,expires_at) VALUES (?,?,?,?,?,?,?,?,?,?,?) ON DUPLICATE KEY UPDATE notification_id=notification_id", vec![
            n.id.clone().into(), n.user_id.into(), n.conversation_id.into(), n.recipient.clone().into(), n.display_name.clone().into(), level.trim_matches('"').to_string().into(), n.risk_audit_id.into(), n.is_test.into(), n.created_at.naive_utc().into(), n.created_at.naive_utc().into(), (n.created_at+chrono::Duration::days(7)).naive_utc().into()
        ]).await? == 1)
    }

    async fn due(&self) -> Result<Vec<RiskMailNotification>, AppError> {
        self.execute("UPDATE risk_mail_notifications SET status='expired' WHERE status='pending' AND expires_at <= UTC_TIMESTAMP(6)", vec![]).await?;
        let rows=self.db.query_all_raw(Statement::from_string(DbBackend::MySql,
            "SELECT notification_id,user_id,conversation_id,recipient,display_name,risk_level,attempts,created_at,risk_audit_id,is_test FROM risk_mail_notifications WHERE status='pending' AND attempts<3 AND next_send_at<=UTC_TIMESTAMP(6) AND expires_at>UTC_TIMESTAMP(6) ORDER BY next_send_at LIMIT 64"))
            .await.map_err(|_| AppError::internal("mail notification fetch failed"))?;
        rows.into_iter()
            .map(|row| {
                let get_error = |_| AppError::internal("invalid stored mail notification");
                let level: String = row.try_get("", "risk_level").map_err(get_error)?;
                let risk_level: RiskLevel =
                    serde_json::from_value(serde_json::Value::String(level))
                        .map_err(|_| AppError::internal("invalid stored mail risk level"))?;
                let created: chrono::NaiveDateTime =
                    row.try_get("", "created_at").map_err(get_error)?;
                Ok(RiskMailNotification {
                    id: row.try_get("", "notification_id").map_err(get_error)?,
                    user_id: row.try_get("", "user_id").map_err(get_error)?,
                    conversation_id: row.try_get("", "conversation_id").map_err(get_error)?,
                    recipient: row.try_get("", "recipient").map_err(get_error)?,
                    display_name: row.try_get("", "display_name").map_err(get_error)?,
                    risk_level,
                    attempts: row.try_get("", "attempts").map_err(get_error)?,
                    created_at: created.and_utc(),
                    risk_audit_id: row.try_get("", "risk_audit_id").map_err(get_error)?,
                    is_test: row.try_get("", "is_test").map_err(get_error)?,
                })
            })
            .collect()
    }

    async fn claim(&self, id: &str) -> Result<bool, AppError> {
        Ok(self.execute("UPDATE risk_mail_notifications SET attempts=attempts+1,last_attempt_at=UTC_TIMESTAMP(6),next_send_at=DATE_ADD(UTC_TIMESTAMP(6),INTERVAL 600 SECOND) WHERE notification_id=? AND status='pending' AND attempts<3 AND next_send_at<=UTC_TIMESTAMP(6) AND expires_at>UTC_TIMESTAMP(6)", vec![id.into()]).await? == 1)
    }
    async fn sent(&self, id: &str, success: bool) -> Result<(), AppError> {
        self.execute("UPDATE risk_mail_notifications SET successful_sends=successful_sends+?,last_sent_at=IF(?,UTC_TIMESTAMP(6),last_sent_at),next_send_at=DATE_ADD(UTC_TIMESTAMP(6),INTERVAL 600 SECOND) WHERE notification_id=?", vec![(success as u8).into(),success.into(),id.into()]).await?;
        Ok(())
    }
    async fn cancel(&self, id: &str) -> Result<(), AppError> {
        self.execute("UPDATE risk_mail_notifications SET status='cancelled' WHERE notification_id=? AND status='pending'", vec![id.into()]).await?;
        Ok(())
    }
    async fn acknowledge(&self, id: &str) -> Result<bool, AppError> {
        self.execute("UPDATE risk_mail_notifications SET status='confirmed',confirmed_at=COALESCE(confirmed_at,UTC_TIMESTAMP(6)) WHERE notification_id=? AND status IN ('pending','confirmed') AND expires_at>UTC_TIMESTAMP(6)", vec![id.into()]).await?;
        let found=self.db.query_one_raw(Statement::from_sql_and_values(DbBackend::MySql,
            "SELECT notification_id FROM risk_mail_notifications WHERE notification_id=? AND status='confirmed' AND expires_at>UTC_TIMESTAMP(6)", vec![Value::from(id)]))
            .await.map_err(|_| AppError::internal("mail confirmation failed"))?;
        Ok(found.is_some())
    }
}
