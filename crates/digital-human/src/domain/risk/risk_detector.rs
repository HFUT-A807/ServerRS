use super::detection_types::DetectionResult;
use crate::shared::error::AppError;
use async_trait::async_trait;

/// 风险检测端口 — 基础设施提供实际的检测器。
#[async_trait]
pub trait RiskDetector: Send + Sync {
    async fn evaluate(&self, text: &str) -> Result<DetectionResult, AppError>;
}
