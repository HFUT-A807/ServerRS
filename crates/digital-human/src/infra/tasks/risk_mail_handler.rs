//! 中高风险邮件：独立有界队列，SMTP 不阻塞共享任务分发器。
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
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
            interval: Duration::from_secs(config.min_interval_secs),
            sent: HashMap::new(),
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
    interval: Duration,
    sent: HashMap<(u64, String), Instant>,
}

impl MailWorker {
    async fn run(mut self, mut receiver: mpsc::Receiver<RiskDetectedTask>) {
        while let Some(event) = receiver.recv().await {
            self.deliver(&event).await;
        }
    }

    async fn deliver(&mut self, event: &RiskDetectedTask) {
        if !should_notify(event.risk_level) {
            return;
        }
        self.sent.retain(|_, time| time.elapsed() < self.interval);
        let key = (event.user_id, format!("{:?}", event.risk_level));
        if self.sent.contains_key(&key) {
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
        let Ok(address) = recipient.parse::<lettre::Address>() else {
            warn!(user_id = event.user_id, "风险邮件跳过：邮箱格式无效");
            return;
        };
        let display_name = user
            .nickname
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(&user.username);
        let Some((subject, body)) = risk_message(event.risk_level, user.id, display_name) else {
            return;
        };
        let message = match Message::builder()
            .from(self.from.clone())
            .to(Mailbox::new(None, address))
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
        {
            Ok(message) => message,
            Err(_) => {
                warn!(user_id = event.user_id, "风险邮件失败：无法构建邮件");
                return;
            }
        };
        for attempt in 1..=3 {
            if matches!(
                tokio::time::timeout(self.timeout, self.sender.send(message.clone())).await,
                Ok(Ok(()))
            ) {
                self.sent.insert(key, Instant::now());
                info!(user_id = event.user_id, conversation_id = ?event.conversation_id,
                    risk_level = ?event.risk_level, "风险提醒邮件已提交 SMTP 服务");
                return;
            }
            warn!(user_id = event.user_id, risk_level = ?event.risk_level, attempt,
                "风险提醒邮件发送失败或超时");
            if attempt < 3 {
                tokio::time::sleep(Duration::from_secs(attempt)).await;
            }
        }
        warn!(
            user_id = event.user_id,
            "风险提醒邮件发送失败：已用尽三次尝试"
        );
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
    use sea_orm::{DbBackend, MockDatabase, Value};
    use std::{collections::BTreeMap, sync::Mutex};

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
            .append_query_results((0..10).map(|_| vec![row.clone()]))
            .into_connection();
        MailWorker {
            user_repo: Arc::new(UserRepo::new(db)),
            sender,
            from: "sender@example.com".parse().unwrap(),
            timeout: Duration::from_secs(1),
            interval: Duration::from_secs(300),
            sent: HashMap::new(),
        }
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
    async fn risk_mail_deduplicates_success_but_allows_escalation_and_expiry() {
        let sender = Arc::new(FakeSender::default());
        let mut worker = worker(Some("owner@example.com"), sender.clone());
        worker.deliver(&event(RiskLevel::Medium)).await;
        worker.deliver(&event(RiskLevel::Medium)).await;
        worker.deliver(&event(RiskLevel::High)).await;
        assert_eq!(sender.messages.lock().unwrap().len(), 2);
        for time in worker.sent.values_mut() {
            *time = Instant::now() - Duration::from_secs(301);
        }
        worker.deliver(&event(RiskLevel::Medium)).await;
        assert_eq!(sender.messages.lock().unwrap().len(), 3);
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
    async fn risk_mail_retries_and_does_not_suppress_events_after_final_failure() {
        let sender = Arc::new(FakeSender::default());
        *sender.failures.lock().unwrap() = 3;
        let mut worker = worker(Some("owner@example.com"), sender.clone());
        worker.deliver(&event(RiskLevel::High)).await;
        assert!(worker.sent.is_empty());
        assert!(sender.messages.lock().unwrap().is_empty());
        worker.deliver(&event(RiskLevel::High)).await;
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
        assert!(config.validate().is_ok());
        assert!(!format!("{config:?}").contains("test-secret-password"));
        config.tls_mode = "none".into();
        assert!(config.validate().is_err());
        config.tls_mode = "tls".into();
        config.from_address = "invalid".into();
        assert!(config.validate().is_err());
    }
}
