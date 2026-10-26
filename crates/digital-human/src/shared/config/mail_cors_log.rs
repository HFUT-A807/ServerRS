use serde::Deserialize;

// ── OllamaConfig ──

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaConfig {
    pub base_url: String,
    #[serde(default = "default_ollama_model")]
    pub model: String,
    #[serde(default = "default_ollama_temperature")]
    pub temperature: f64,
    #[serde(default = "default_ollama_top_p")]
    pub top_p: f64,
    #[serde(default)]
    pub tunnel: Option<String>,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            model: default_ollama_model(),
            temperature: default_ollama_temperature(),
            top_p: default_ollama_top_p(),
            tunnel: None,
        }
    }
}

fn default_ollama_model() -> String {
    "qwen2.5:14b".into()
}
fn default_ollama_temperature() -> f64 {
    0.5
}
fn default_ollama_top_p() -> f64 {
    0.9
}

// ── DetectorConfig ──

#[derive(Debug, Clone, Deserialize)]
pub struct DetectorConfig {
    #[serde(default = "default_context_window")]
    pub context_window_size: usize,
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f64,
    #[serde(default)]
    pub llm_enabled: bool,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            context_window_size: default_context_window(),
            confidence_threshold: default_confidence_threshold(),
            llm_enabled: false,
            max_retries: default_max_retries(),
        }
    }
}

fn default_context_window() -> usize {
    4
}
fn default_confidence_threshold() -> f64 {
    0.5
}
fn default_max_retries() -> u32 {
    3
}

// ── MailConfig ──

#[derive(Clone, Deserialize)]
pub struct MailConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_mail_host")]
    pub host: String,
    #[serde(default = "default_mail_port")]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_mail_from")]
    pub from_address: String,
    /// starttls（通常 587）或 tls（通常 465），均强制加密。
    #[serde(default = "default_mail_tls")]
    pub tls_mode: String,
    #[serde(default = "default_mail_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_mail_interval")]
    pub min_interval_secs: u64,
    /// Public origin used in guardian confirmation links.
    #[serde(default)]
    pub public_base_url: String,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_mail_host(),
            port: default_mail_port(),
            username: String::new(),
            password: String::new(),
            from_address: default_mail_from(),
            tls_mode: default_mail_tls(),
            timeout_secs: default_mail_timeout(),
            min_interval_secs: default_mail_interval(),
            public_base_url: String::new(),
        }
    }
}

impl std::fmt::Debug for MailConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MailConfig")
            .field("enabled", &self.enabled)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .field("from_address", &self.from_address)
            .field("tls_mode", &self.tls_mode)
            .field("timeout_secs", &self.timeout_secs)
            .field("min_interval_secs", &self.min_interval_secs)
            .field("public_base_url", &self.public_base_url)
            .finish()
    }
}

impl MailConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        let url = url::Url::parse(&self.public_base_url)
            .map_err(|_| "mail.public_base_url must be a public HTTP(S) origin")?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
        {
            return Err("mail.public_base_url must be an HTTP(S) origin without path, credentials, query or fragment".into());
        }
        if self.host.trim().is_empty() || self.host == "smtp.example.com" || self.port == 0 {
            return Err("mail.host and mail.port must specify a real SMTP server".into());
        }
        if self.username.trim().is_empty() || self.password.is_empty() {
            return Err("mail.username and mail.password are required when mail is enabled".into());
        }
        if self
            .from_address
            .parse::<lettre::message::Mailbox>()
            .is_err()
        {
            return Err("mail.from_address must be a valid mailbox".into());
        }
        if !matches!(self.tls_mode.as_str(), "starttls" | "tls") {
            return Err("mail.tls_mode must be starttls or tls".into());
        }
        if self.timeout_secs == 0 || self.timeout_secs > 120 || self.min_interval_secs == 0 {
            return Err(
                "mail.timeout_secs must be 1..120 and mail.min_interval_secs must be positive"
                    .into(),
            );
        }
        Ok(())
    }
}

fn default_mail_tls() -> String {
    "starttls".into()
}
fn default_mail_timeout() -> u64 {
    15
}
fn default_mail_interval() -> u64 {
    300
}

fn default_mail_host() -> String {
    "smtp.example.com".into()
}
fn default_mail_port() -> u16 {
    587
}
fn default_mail_from() -> String {
    "noreply@example.com".into()
}

// ── CorsConfig ──

#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: default_allowed_origins(),
        }
    }
}

fn default_allowed_origins() -> Vec<String> {
    vec!["http://localhost:3000".into()]
}

// ── LoggingConfig ──

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

fn default_log_level() -> String {
    "info".into()
}
