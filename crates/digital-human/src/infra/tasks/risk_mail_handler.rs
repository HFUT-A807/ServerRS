//! 中高风险邮件：独立有界队列，SMTP 不阻塞共享任务分发器。
use std::{sync::Arc, time::Duration};

use crate::domain::risk::risk_mail_notification::{
    RiskMailLinkT, RiskMailNotification, RiskMailRepoT,
};
use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{
    domain::{
        risk::detection_types::RiskLevel,
        tasks::{
            task_event::{RiskDetectedTask, TaskEvent},
            task_handler::TaskHandler,
        },
        user::user_repo::UserRepoT,
    },
    shared::config::MailConfig,
};

#[async_trait]
trait MailSender: Send + Sync {
    async fn send(&self, message: Message) -> Result<(), ()>;
}

struct SmtpSender(AsyncSmtpTransport<Tokio1Executor>);

#[async_trait]
impl MailSender for SmtpSender {
    async fn send(&self, message: Message) -> Result<(), ()> {
        // 不记录 SMTP 原始错误，避免泄漏地址、凭据或服务端返回内容。
        self.0.send(message).await.map(|_| ()).map_err(|_| ())
    }
}

pub struct RiskMailHandler {
    queue: mpsc::Sender<RiskDetectedTask>,
}

impl RiskMailHandler {
    /// 由启动层持有并管理返回的后台任务；关闭时随其他任务一同停止。
    pub fn start(
        config: &MailConfig,
        user_repo: Arc<dyn UserRepoT>,
        repo: Arc<dyn RiskMailRepoT>,
        confirmation: Arc<dyn RiskMailLinkT>,
    ) -> Result<(Self, tokio::task::JoinHandle<()>), String> {
        config.validate()?;
        if !config.enabled {
            return Err("risk mail is disabled".into());
        }
        let builder = match config.tls_mode.as_str() {
            "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host),
            _ => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host),
        }
        .map_err(|_| "invalid SMTP TLS configuration")?;
        let sender = SmtpSender(
            builder
                .port(config.port)
                .credentials(Credentials::new(
                    config.username.clone(),
                    config.password.clone(),
                ))
                .timeout(Some(Duration::from_secs(config.timeout_secs)))
                .build(),
        );
        let worker = MailWorker {
            user_repo,
            sender: Arc::new(sender),
            from: config
                .from_address
                .parse()
                .map_err(|_| "invalid mail.from_address")?,
            timeout: Duration::from_secs(config.timeout_secs),
            repo,
            confirmation,
        };
        let (queue, receiver) = mpsc::channel(256);
        let handle = tokio::spawn(worker.run(receiver));
        info!("风险邮件提醒已启用（Medium/High/Crisis）");
        Ok((Self { queue }, handle))
    }
}

#[async_trait]
impl TaskHandler for RiskMailHandler {
    fn name(&self) -> &str {
        "RiskMailHandler"
    }

    async fn handle(&self, event: &TaskEvent) {
        if let TaskEvent::RiskDetected(event) = event {
            if !should_notify(event.risk_level) {
                return;
            }
            if self.queue.try_send(event.clone()).is_err() {
                warn!(user_id = event.user_id, risk_level = ?event.risk_level,
                    "风险邮件未入队：队列已满或关闭");
            }
        }
    }
}

fn should_notify(level: RiskLevel) -> bool {
    matches!(
        level,
        RiskLevel::Medium | RiskLevel::High | RiskLevel::Crisis
    )
}

struct MailWorker {
    user_repo: Arc<dyn UserRepoT>,
    sender: Arc<dyn MailSender>,
    from: Mailbox,
    timeout: Duration,
    repo: Arc<dyn RiskMailRepoT>,
    confirmation: Arc<dyn RiskMailLinkT>,
}

impl MailWorker {
    async fn run(mut self, mut receiver: mpsc::Receiver<RiskDetectedTask>) {
        let mut timer = tokio::time::interval(Duration::from_secs(5));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                event=receiver.recv() => match event {
                    Some(event) => self.deliver(&event).await,
                    None => break,
                },
                _=timer.tick() => self.process_due().await,
            }
        }
    }

    async fn deliver(&mut self, event: &RiskDetectedTask) {
        if !should_notify(event.risk_level) {
            return;
        }
        let user = match self.user_repo.find_by_id(event.user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                warn!(user_id = event.user_id, "风险邮件跳过：用户不存在");
                return;
            }
            Err(_) => {
                warn!(user_id = event.user_id, "风险邮件失败：无法读取监护人邮箱");
                return;
            }
        };
        // 当前业务约定：用户账号的 email 字段登记的是其监护人的收件邮箱。
        let recipient = user
            .email
            .as_deref()
            .map(str::trim)
            .filter(|email| !email.is_empty());
        let Some(recipient) = recipient else {
            warn!(user_id = event.user_id, "风险邮件跳过：未登记监护人邮箱");
            return;
        };
        // 仅允许一个纯邮箱地址；不接受显示名、地址列表或邮件头。
        let Ok(_address) = recipient.parse::<lettre::Address>() else {
            warn!(user_id = event.user_id, "风险邮件跳过：邮箱格式无效");
            return;
        };
        let display_name = user
            .nickname
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(&user.username);
        let notification = RiskMailNotification {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user.id,
            conversation_id: event.conversation_id,
            recipient: recipient.into(),
            display_name: display_name.chars().take(80).collect(),
            risk_level: event.risk_level,
            attempts: 0,
            created_at: chrono::Utc::now(),
            risk_audit_id: None,
            is_test: false,
        };
        if self.repo.enqueue(&notification).await.is_err() {
            warn!(user_id = event.user_id, "风险邮件入库失败");
            return;
        }
        self.process_due().await;
    }

    async fn process_due(&mut self) {
        let notifications = match self.repo.due().await {
            Ok(records) => records,
            Err(_) => {
                warn!("风险邮件待发送记录读取失败");
                return;
            }
        };
        for n in notifications {
            // A changed/deleted guardian contact must not receive stale scheduled mail.
            let user = match self.user_repo.find_by_id(n.user_id).await {
                Ok(Some(user)) => user,
                Ok(None) => {
                    let _ = self.repo.cancel(&n.id).await;
                    continue;
                }
                Err(_) => {
                    warn!(notification_id=%n.id,"风险邮件用户读取失败");
                    continue;
                }
            };
            if !n.is_test && user.email.as_deref().map(str::trim) != Some(n.recipient.as_str()) {
                let _ = self.repo.cancel(&n.id).await;
                continue;
            }
            self.send_notification(&n).await;
        }
    }

    async fn send_notification(&self, n: &RiskMailNotification) {
        let Some((subject, mut body)) = risk_message(n.risk_level, n.user_id, &n.display_name)
        else {
            return;
        };
        let Ok(address) = n.recipient.parse::<lettre::Address>() else {
            let _ = self.repo.cancel(&n.id).await;
            return;
        };
        let url = match self.confirmation.confirmation_url(&n.id, n.created_at) {
            Ok(url) => url,
            Err(_) => {
                warn!(notification_id=%n.id,"风险邮件确认链接生成失败");
                return;
            }
        };
        if n.is_test {
            body = format!(
                "【功能验证邮件：使用数据库中的历史真实风险记录，非新增风险事件。风险审计编号：{}】\n\n{body}",
                n.risk_audit_id.map(|id| id.to_string()).unwrap_or_default()
            );
        }
        body.push_str(&format!("\n\n本次为第 {} 次提醒（累计最多 3 次）。请打开以下链接，并点击页面上的“确认收到”：\n{url}\n\n确认后将停止本条提醒的后续补发。未确认时，每隔 10 分钟补发一次；直接回复邮件不会停止补发。链接有效期为 7 天。",n.attempts+1));
        let message = match Message::builder()
            .from(self.from.clone())
            .to(Mailbox::new(None, address))
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)
        {
            Ok(message) => message,
            Err(_) => {
                warn!(notification_id=%n.id, "风险邮件失败：无法构建邮件");
                return;
            }
        };
        // Reserve before SMTP: concurrent workers/restarts cannot reset or exceed the limit.
        match self.repo.claim(&n.id).await {
            Ok(true) => {}
            Ok(false) => return,
            Err(_) => {
                warn!(notification_id=%n.id,"风险邮件发送次数预留失败");
                return;
            }
        }
        let success = matches!(
            tokio::time::timeout(self.timeout, self.sender.send(message)).await,
            Ok(Ok(()))
        );
        if self.repo.sent(&n.id, success).await.is_err() {
            warn!(notification_id=%n.id,"风险邮件投递结果保存失败");
        }
        if success {
            info!(notification_id=%n.id,user_id=n.user_id,attempt=n.attempts+1,"风险提醒邮件已提交 SMTP 服务");
        } else {
            warn!(notification_id=%n.id,attempt=n.attempts+1,"风险邮件失败或超时；按十分钟间隔处理，累计最多三次");
        }
    }
}

fn risk_message(
    level: RiskLevel,
    user_id: u64,
    display_name: &str,
) -> Option<(&'static str, String)> {
    let (label, advice) = match level {
        RiskLevel::Medium => (
            "中风险",
            "系统在被监护人的近期对话中识别到可能的情绪困扰。建议您及时与其联系，以耐心、平和的方式了解近况，给予关心和陪伴。如困扰持续或影响日常生活，建议协助其寻求专业心理或医疗支持。",
        ),
        RiskLevel::High => (
            "高风险",
            "系统在被监护人的近期对话中识别到可能的较高心理或安全风险。请您尽快与其取得联系，确认当前状态和人身安全，必要时安排可信任的人陪伴，并协助其尽快获得专业心理或医疗支持。如存在立即受到伤害的危险，请立即联系当地紧急救援服务或前往最近的急诊。",
        ),
        RiskLevel::Crisis => (
            "危机",
            "系统在被监护人的近期对话中识别到可能的紧迫安全风险。请您立即尝试联系并确认其人身安全，尽可能安排可信任的人到场陪伴。如存在立即受到伤害的危险，请立即联系当地紧急救援服务或前往最近的急诊；如无法联系被监护人且担心其安全，请尽快寻求当地紧急救援服务协助。请勿等待后续邮件或邮件回复后再采取行动。",
        ),
        _ => return None,
    };
    // 名称仅作单行身份提示，避免控制字符改变通知结构。
    let name: String = display_name
        .trim()
        .chars()
        .take(80)
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    let body = format!(
        "尊敬的监护人：\n\n您收到此邮件，是因为该邮箱被登记为下列用户的监护人联系邮箱。\n\n被监护人：{name}（用户ID：{user_id}）\n系统识别等级：{label}\n\n{advice}\n\n此提醒由系统基于对话自动识别生成，可能存在误判，不构成医学诊断，请结合实际情况核实。请尊重被监护人的隐私，以支持和沟通的方式跟进。此邮箱不提供实时支持。\n\nServerRS 风险提醒"
    );
    // 主题不暴露身份或等级，正文不包含对话原文或模型分析。
    Some(("监护人风险提醒", body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::repo::seaorm_impl::user::UserRepo;

    #[test]
    fn risk_mail_templates_address_guardians_and_identify_the_ward() {
        for (level, label, action) in [
            (RiskLevel::Medium, "中风险", "及时与其联系"),
            (RiskLevel::High, "高风险", "尽快与其取得联系"),
            (RiskLevel::Crisis, "危机", "立即尝试联系"),
        ] {
            let (subject, body) = risk_message(level, 7, "小明").unwrap();
            assert_eq!(subject, "监护人风险提醒");
            assert!(body.starts_with("尊敬的监护人："));
            assert!(body.contains("被监护人：小明（用户ID：7）"));
            assert!(body.contains(&format!("系统识别等级：{label}")));
            assert!(body.contains(action));
            assert!(body.contains("可能存在误判"));
            assert!(!body.contains("你可能正在"));
        }
        for level in [RiskLevel::None, RiskLevel::Low, RiskLevel::Unknown] {
            assert!(risk_message(level, 7, "小明").is_none());
        }
        let (_, body) = risk_message(RiskLevel::High, 7, "小明\r\n附加内容").unwrap();
        assert!(body.contains("被监护人：小明  附加内容（用户ID：7）"));
    }
    use crate::shared::error::AppError;
    use sea_orm::{DbBackend, MockDatabase, Value};
    use std::{collections::BTreeMap, sync::Mutex};

    #[derive(Default)]
    struct MemoryRepo {
        state: Mutex<MemoryState>,
    }
    #[derive(Default)]
    struct MemoryState {
        now: i64,
        records: Vec<(RiskMailNotification, i64, bool)>,
    }
    impl MemoryRepo {
        fn advance(&self, seconds: i64) {
            self.state.lock().unwrap().now += seconds;
        }
        fn count(&self) -> u8 {
            self.state.lock().unwrap().records[0].0.attempts
        }
    }
    #[async_trait]
    impl RiskMailRepoT for MemoryRepo {
        async fn enqueue(&self, n: &RiskMailNotification) -> Result<bool, AppError> {
            let mut state = self.state.lock().unwrap();
            if state.records.iter().any(|(r, _, closed)| {
                !closed
                    && r.user_id == n.user_id
                    && r.risk_level == n.risk_level
                    && r.recipient == n.recipient
            }) {
                return Ok(false);
            }
            state
                .records
                .push((n.clone(), n.created_at.timestamp(), false));
            Ok(true)
        }
        async fn due(&self) -> Result<Vec<RiskMailNotification>, AppError> {
            let state = self.state.lock().unwrap();
            Ok(state
                .records
                .iter()
                .filter(|(r, next, closed)| !closed && r.attempts < 3 && *next <= state.now)
                .map(|(r, _, _)| r.clone())
                .collect())
        }
        async fn claim(&self, id: &str) -> Result<bool, AppError> {
            let mut state = self.state.lock().unwrap();
            let now = state.now;
            let Some((r, next, closed)) = state.records.iter_mut().find(|(r, _, _)| r.id == id)
            else {
                return Ok(false);
            };
            if *closed || r.attempts >= 3 || *next > now {
                return Ok(false);
            }
            r.attempts += 1;
            *next = now + 600;
            Ok(true)
        }
        async fn sent(&self, _: &str, _: bool) -> Result<(), AppError> {
            Ok(())
        }
        async fn cancel(&self, id: &str) -> Result<(), AppError> {
            self.acknowledge(id).await.map(|_| ())
        }
        async fn acknowledge(&self, id: &str) -> Result<bool, AppError> {
            let mut state = self.state.lock().unwrap();
            let Some((_, _, closed)) = state.records.iter_mut().find(|(r, _, _)| r.id == id) else {
                return Ok(false);
            };
            *closed = true;
            Ok(true)
        }
    }
    struct TestLinks;
    impl RiskMailLinkT for TestLinks {
        fn confirmation_url(
            &self,
            id: &str,
            _: chrono::DateTime<chrono::Utc>,
        ) -> Result<String, AppError> {
            Ok(format!("https://example.test/mail/confirm#{id}"))
        }
    }

    #[derive(Default)]
    struct FakeSender {
        messages: Mutex<Vec<Message>>,
        failures: Mutex<usize>,
    }

    #[async_trait]
    impl MailSender for FakeSender {
        async fn send(&self, message: Message) -> Result<(), ()> {
            let mut failures = self.failures.lock().unwrap();
            if *failures > 0 {
                *failures -= 1;
                return Err(());
            }
            self.messages.lock().unwrap().push(message);
            Ok(())
        }
    }

    fn worker(email: Option<&str>, sender: Arc<FakeSender>) -> MailWorker {
        scheduled_worker(email, sender).0
    }

    fn scheduled_worker(
        email: Option<&str>,
        sender: Arc<FakeSender>,
    ) -> (MailWorker, Arc<MemoryRepo>) {
        let now = chrono::Utc::now().naive_utc();
        let row: BTreeMap<&str, Value> = [
            ("id", 7_u64.into()),
            ("username", "test".into()),
            ("password", "unused".into()),
            ("email", email.map(str::to_owned).into()),
            ("phone", Option::<String>::None.into()),
            ("avatar", Option::<Vec<u8>>::None.into()),
            ("nickname", Option::<String>::None.into()),
            ("created_at", now.into()),
            ("updated_at", now.into()),
            (
                "last_login_at",
                Option::<chrono::NaiveDateTime>::None.into(),
            ),
            ("status", 1_i8.into()),
            ("role", "USER".into()),
        ]
        .into_iter()
        .collect();
        let db = MockDatabase::new(DbBackend::MySql)
            .append_query_results((0..100).map(|_| vec![row.clone()]))
            .into_connection();
        let repo = Arc::new(MemoryRepo::default());
        repo.state.lock().unwrap().now = chrono::Utc::now().timestamp();
        (
            MailWorker {
                user_repo: Arc::new(UserRepo::new(db)),
                sender,
                from: "sender@example.com".parse().unwrap(),
                timeout: Duration::from_secs(1),
                repo: repo.clone(),
                confirmation: Arc::new(TestLinks),
            },
            repo,
        )
    }

    fn event(level: RiskLevel) -> RiskDetectedTask {
        RiskDetectedTask {
            user_id: 7,
            conversation_id: Some(42),
            risk_level: level,
            confidence: 0.9,
        }
    }

    #[tokio::test]
    async fn risk_mail_routes_only_medium_high_crisis_and_uses_registered_email() {
        let sender = Arc::new(FakeSender::default());
        let mut worker = worker(Some("owner@example.com"), sender.clone());
        for level in [
            RiskLevel::None,
            RiskLevel::Unknown,
            RiskLevel::Low,
            RiskLevel::Medium,
            RiskLevel::High,
            RiskLevel::Crisis,
        ] {
            worker.deliver(&event(level)).await;
        }
        let messages = sender.messages.lock().unwrap();
        assert_eq!(messages.len(), 3);
        for message in messages.iter() {
            assert_eq!(message.envelope().to()[0].to_string(), "owner@example.com");
            assert_eq!(message.envelope().to().len(), 1);
        }
    }

    #[tokio::test]
    async fn risk_mail_waits_ten_minutes_stops_at_three_and_allows_escalation() {
        let sender = Arc::new(FakeSender::default());
        let (mut worker, repo) = scheduled_worker(Some("owner@example.com"), sender.clone());
        worker.deliver(&event(RiskLevel::Medium)).await;
        worker.deliver(&event(RiskLevel::Medium)).await;
        worker.deliver(&event(RiskLevel::High)).await;
        assert_eq!(sender.messages.lock().unwrap().len(), 2);
        repo.advance(599);
        worker.process_due().await;
        assert_eq!(sender.messages.lock().unwrap().len(), 2);
        repo.advance(1);
        worker.process_due().await;
        assert_eq!(sender.messages.lock().unwrap().len(), 4);
        repo.advance(600);
        worker.process_due().await;
        assert_eq!(sender.messages.lock().unwrap().len(), 6);
        repo.advance(6000);
        worker.process_due().await;
        worker.deliver(&event(RiskLevel::Medium)).await;
        assert_eq!(sender.messages.lock().unwrap().len(), 6);
        assert_eq!(repo.count(), 3);
    }

    #[tokio::test]
    async fn risk_mail_skips_missing_or_invalid_addresses() {
        let sender = Arc::new(FakeSender::default());
        for email in [
            None,
            Some(""),
            Some("not-an-email"),
            Some("a@example.com, b@example.com"),
            Some("a@example.com\r\nBcc: b@example.com"),
        ] {
            worker(email, sender.clone())
                .deliver(&event(RiskLevel::High))
                .await;
        }
        assert!(sender.messages.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn risk_mail_failed_attempts_are_spaced_and_capped() {
        let sender = Arc::new(FakeSender::default());
        *sender.failures.lock().unwrap() = 3;
        let (mut worker, repo) = scheduled_worker(Some("owner@example.com"), sender.clone());
        worker.deliver(&event(RiskLevel::High)).await;
        assert_eq!(repo.count(), 1);
        assert!(sender.messages.lock().unwrap().is_empty());
        repo.advance(600);
        worker.process_due().await;
        repo.advance(600);
        worker.process_due().await;
        repo.advance(600);
        worker.process_due().await;
        assert_eq!(repo.count(), 3);
        assert!(sender.messages.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn risk_mail_confirmation_stops_future_reminders_after_worker_restart() {
        let sender = Arc::new(FakeSender::default());
        let (mut first, repo) = scheduled_worker(Some("owner@example.com"), sender.clone());
        first.deliver(&event(RiskLevel::High)).await;
        let id = repo.state.lock().unwrap().records[0].0.id.clone();
        assert!(repo.acknowledge(&id).await.unwrap());
        assert!(repo.acknowledge(&id).await.unwrap());
        let mut restarted = worker(Some("owner@example.com"), sender.clone());
        restarted.repo = repo.clone();
        repo.advance(1200);
        restarted.process_due().await;
        assert_eq!(sender.messages.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn risk_mail_handler_enqueues_without_waiting_for_smtp() {
        let (queue, mut receiver) = mpsc::channel(1);
        let handler = RiskMailHandler { queue };
        handler
            .handle(&TaskEvent::RiskDetected(event(RiskLevel::Low)))
            .await;
        assert!(receiver.try_recv().is_err());
        handler
            .handle(&TaskEvent::RiskDetected(event(RiskLevel::Medium)))
            .await;
        // 满队列不会阻塞分发器。
        handler
            .handle(&TaskEvent::RiskDetected(event(RiskLevel::High)))
            .await;
        assert_eq!(receiver.try_recv().unwrap().risk_level, RiskLevel::Medium);
    }

    #[test]
    fn risk_mail_config_requires_credentials_tls_and_redacts_password() {
        let mut config = MailConfig::default();
        assert!(config.validate().is_ok());
        config.enabled = true;
        assert!(config.validate().is_err());
        config.host = "smtp.test.invalid".into();
        config.username = "sender".into();
        config.password = "test-secret-password".into();
        config.public_base_url = "https://example.test".into();
        assert!(config.validate().is_ok());
        assert!(!format!("{config:?}").contains("test-secret-password"));
        config.tls_mode = "none".into();
        assert!(config.validate().is_err());
        config.tls_mode = "tls".into();
        config.from_address = "invalid".into();
        assert!(config.validate().is_err());
    }
}
