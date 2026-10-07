use std::sync::Arc;

use crate::app::risk::risk_detection_service::RiskDetectionService;
use crate::domain::risk::risk_detector::RiskDetector;
use crate::domain::tasks::task_publisher::TaskPublisher;
use crate::infra::detector::ollama_risk_detector::OllamaRiskDetector;

use super::BootstrapContext;

pub(crate) fn build_risk_detection_service(
    ctx: &BootstrapContext<'_>,
    task_publisher: Arc<dyn TaskPublisher>,
) -> Arc<RiskDetectionService> {
    let risk_detector: Arc<dyn RiskDetector> = Arc::new(OllamaRiskDetector::new(Arc::clone(
        &ctx.infra.ollama_provider,
    )));
    Arc::new(RiskDetectionService::new(
        Arc::clone(&ctx.repos.risk_repo),
        task_publisher,
        risk_detector,
        ctx.config.llm.chat_model.clone(),
    ))
}
