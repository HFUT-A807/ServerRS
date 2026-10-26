use std::sync::Arc;

use crate::app::risk::risk_mail_confirmation_service::RiskMailConfirmationService;
use crate::domain::risk::risk_mail_notification::{RiskMailLinkT, RiskMailRepoT};
use crate::domain::tasks::task_handler::TaskHandler;
use crate::domain::tasks::task_publisher::TaskPublisher;
use crate::domain::user::user_repo::UserRepoT;
use crate::infra::tasks::alert_handler::{AlertConfig, AlertHandler};
use crate::infra::tasks::in_memory_task_flow::{
    RetryingTaskPublisher, TaskWorker, new_task_channel,
};
use crate::infra::tasks::logging_handler::LoggingHandler;
use crate::infra::tasks::rate_limit_handler::{RateLimitConfig, RateLimitHandler};
use crate::infra::tasks::risk_mail_handler::RiskMailHandler;
use crate::infra::tasks::risk_mail_store::RiskMailStore;
use crate::shared::config::MailConfig;

pub struct BackgroundTasks {
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl BackgroundTasks {
    pub fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    pub fn spawn(&mut self, handle: tokio::task::JoinHandle<()>) {
        self.handles.push(handle);
    }

    pub fn abort_all(self) {
        for handle in self.handles {
            handle.abort();
        }
    }
}

impl Default for BackgroundTasks {
    fn default() -> Self {
        Self::new()
    }
}

/// 任务系统上下文。前半段在业务服务之前构建（无服务依赖），
/// 后半段通过 `post_service_setup()` 在服务就绪后完成。
#[allow(dead_code)]
pub struct TaskContext {
    pub task_publisher: Arc<dyn TaskPublisher>,
    pub background: BackgroundTasks,
    pub mail_confirmation: Option<Arc<RiskMailConfirmationService>>,
    task_worker: Option<TaskWorker>,
    alert_handler: Arc<AlertHandler>,
    rate_limit_handler: Arc<RateLimitHandler>,
}

impl TaskContext {
    /// 构造任务系统前半段：告警、限流、通道、Publisher。
    pub async fn new(
        user_repo: Arc<dyn UserRepoT>,
        mail: &MailConfig,
        db: &sea_orm::DatabaseConnection,
        confirmation_secret: &str,
    ) -> Result<Self, std::io::Error> {
        let mut background = BackgroundTasks::new();

        let alert_handler = Arc::new(AlertHandler::new(AlertConfig::default()));
        let rate_limit_handler = Arc::new(RateLimitHandler::new(
            RateLimitConfig::default(),
            Arc::clone(&user_repo),
        ));

        let mut mail_confirmation = None;
        let mail_handler = if mail.enabled {
            mail.validate().map_err(std::io::Error::other)?;
            let store = Arc::new(RiskMailStore::new(db.clone()));
            store
                .ensure_schema()
                .await
                .map_err(|_| std::io::Error::other("risk mail schema initialization failed"))?;
            let repo: Arc<dyn RiskMailRepoT> = store;
            let confirmation = Arc::new(RiskMailConfirmationService::new(
                Arc::clone(&repo),
                confirmation_secret.into(),
                mail.public_base_url.clone(),
            ));
            mail_confirmation = Some(Arc::clone(&confirmation));
            let links: Arc<dyn RiskMailLinkT> = confirmation;
            let (handler, handle) = RiskMailHandler::start(mail, user_repo, repo, links)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
            background.spawn(handle);
            Some(handler)
        } else {
            tracing::info!("风险邮件提醒未启用（mail.enabled=false）");
            None
        };

        let (tp, tw) = new_task_channel(256);
        let _retry_handle = RetryingTaskPublisher::spawn_retry_worker(tp.clone());
        let mut task_worker = tw
            .with_handler(Arc::new(LoggingHandler))
            .with_handler(Arc::clone(&alert_handler) as Arc<dyn TaskHandler>)
            .with_handler(Arc::clone(&rate_limit_handler) as Arc<dyn TaskHandler>);
        if let Some(handler) = mail_handler {
            task_worker = task_worker.with_handler(Arc::new(handler));
        }

        // 有状态处理器的定期清理
        {
            let h = Arc::clone(&alert_handler);
            background.spawn(tokio::spawn(async move {
                let mut i = tokio::time::interval(tokio::time::Duration::from_secs(300));
                loop {
                    i.tick().await;
                    h.cleanup().await;
                }
            }));
        }
        {
            let h = Arc::clone(&rate_limit_handler);
            background.spawn(tokio::spawn(async move {
                let mut i = tokio::time::interval(tokio::time::Duration::from_secs(120));
                loop {
                    i.tick().await;
                    h.cleanup().await;
                }
            }));
        }

        let task_publisher: Arc<dyn TaskPublisher> = Arc::new(tp);

        Ok(Self {
            task_publisher,
            background,
            mail_confirmation,
            task_worker: Some(task_worker),
            alert_handler,
            rate_limit_handler,
        })
    }

    /// 后半段注册：在业务服务就绪后调用。注册依赖服务的 handler 并启动 worker。
    pub fn start_service_handlers(
        &mut self,
        risk_audit_worker: Arc<dyn TaskHandler>,
        summary_refresh_handler: Arc<dyn TaskHandler>,
    ) {
        if let Some(worker) = self.task_worker.take() {
            self.background.spawn(tokio::spawn(
                worker
                    .with_handler(risk_audit_worker)
                    .with_handler(summary_refresh_handler)
                    .run(),
            ));
        }
    }
}
