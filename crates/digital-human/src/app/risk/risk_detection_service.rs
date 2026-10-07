use std::sync::Arc;

use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use crate::domain::risk::detection_types::RiskLevel;
use crate::domain::risk::post_conversation_risk_audit::{
    NewPostConversationRiskAudit, PostRiskAuditResult,
};
use crate::domain::risk::risk_detector::RiskDetector;
use crate::domain::risk::risk_repo::RiskRepoT;
use crate::domain::tasks::task_event::{RiskDetectedTask, TaskEvent};
use crate::domain::tasks::task_publisher::TaskPublisher;
use crate::shared::error::AppError;

/// Runs risk detection only for already-persisted, closed conversation turns.
pub struct RiskDetectionService {
    risk_repo: Arc<dyn RiskRepoT>,
    task_publisher: Arc<dyn TaskPublisher>,
    detector: Arc<dyn RiskDetector>,
    model_name: String,
}

impl RiskDetectionService {
    pub fn new(
        risk_repo: Arc<dyn RiskRepoT>,
        task_publisher: Arc<dyn TaskPublisher>,
        detector: Arc<dyn RiskDetector>,
        model_name: String,
    ) -> Self {
        Self {
            risk_repo,
            task_publisher,
            detector,
            model_name,
        }
    }

    pub async fn audit_closed_turn(
        &self,
        user_id: u64,
        conversation_id: u64,
        user_message_id: u64,
        assistant_message_id: u64,
        canonical_input: String,
    ) -> Result<(), AppError> {
        let audit = self
            .risk_repo
            .create_pending(NewPostConversationRiskAudit {
                user_id,
                conversation_id,
                audit_scope: "turn".into(),
                user_message_ref_id: Some(user_message_id),
                assistant_message_ref_id: Some(assistant_message_id),
                user_message_id: Some(user_message_id),
                assistant_message_id: Some(assistant_message_id),
            })
            .await?;
        self.risk_repo.mark_running(audit.audit_id).await?;

        // 模型调用失败不能按无风险入库，否则会掩盖一次未完成的审计。
        let result = match self.detector.evaluate(&canonical_input).await {
            Ok(result) => result,
            Err(error) => {
                self.risk_repo
                    .mark_failed(audit.audit_id, error.to_string())
                    .await?;
                return Err(error);
            }
        };

        let input_hash = Sha256::digest(canonical_input.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let risk_level = serde_json::to_string(&result.risk_level)
            .unwrap_or_else(|_| "\"unknown\"".into())
            .trim_matches('"')
            .to_lowercase();
        self.risk_repo
            .mark_completed(
                audit.audit_id,
                PostRiskAuditResult {
                    risk_level,
                    risk_categories: None,
                    confidence: Some(result.confidence),
                    input_hash: Some(input_hash),
                    detector_name: Some("ollama-semantic".into()),
                    detector_version: Some("1.0".into()),
                    model_name: Some(self.model_name.clone()),
                    checked_at: chrono::Utc::now(),
                },
            )
            .await?;

        let actionable =
            result.risk_level != RiskLevel::None && result.risk_level != RiskLevel::Unknown;
        if actionable {
            info!(
                user_id,
                conversation_id,
                risk_level = ?result.risk_level,
                confidence = result.confidence,
                "post-conversation risk audit detected an actionable level"
            );
            if let Err(error) = self
                .task_publisher
                .publish(TaskEvent::RiskDetected(RiskDetectedTask {
                    user_id,
                    conversation_id: Some(conversation_id),
                    risk_level: result.risk_level,
                    confidence: result.confidence,
                }))
                .await
            {
                warn!(
                    user_id,
                    conversation_id,
                    %error,
                    "risk audit completed, but the follow-up notification could not be published"
                );
            }
        } else {
            debug!(
                user_id,
                conversation_id,
                risk_level = ?result.risk_level,
                "post-conversation risk audit completed"
            );
        }

        Ok(())
    }
}
