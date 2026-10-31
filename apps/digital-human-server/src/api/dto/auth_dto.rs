use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::app::auth::auth_service::validate_registration_password;

fn validate_registration_password_field(password: &str) -> Result<(), validator::ValidationError> {
    validate_registration_password(password)
        .map_err(|_| validator::ValidationError::new("password_policy"))
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 3, max = 32))]
    pub username: String,
    // 兼容历史账号的短密码哈希；新账号注册仍执行至少 8 位的安全限制。
    #[validate(length(max = 128))]
    pub password: String,
    #[validate(length(max = 64))]
    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user_id: u64,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 32), regex(path = "*USERNAME_RE"))]
    pub username: String,
    #[validate(custom(function = "validate_registration_password_field"))]
    pub password: String,
    #[validate(length(max = 64))]
    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: u64,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RefreshTokenRequest {
    #[validate(length(min = 32, max = 4096))]
    pub refresh_token: String,
    #[validate(length(max = 64))]
    #[serde(default)]
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LogoutRequest {
    #[validate(length(min = 32, max = 4096))]
    pub refresh_token: String,
    #[validate(length(max = 128))]
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

static USERNAME_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^[a-zA-Z0-9_]+$").expect("valid username regex")
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_accepts_a_legacy_short_password() {
        let request = LoginRequest {
            username: "test".into(),
            password: "123123".into(),
            device_id: None,
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn registration_still_rejects_a_short_password() {
        let request = RegisterRequest {
            username: "new_user".into(),
            password: "123123".into(),
            device_id: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn registration_rejects_missing_password_character_classes() {
        for password in ["abcdefgh!", "ABCDEFGH!", "Abcdefgh", "Abcdefg "] {
            let request = RegisterRequest {
                username: "new_user".into(),
                password: password.into(),
                device_id: None,
            };
            assert!(request.validate().is_err());
        }
    }

    #[test]
    fn registration_accepts_an_eight_character_password_without_digits() {
        let request = RegisterRequest {
            username: "new_user".into(),
            password: "Abcdefg!".into(),
            device_id: None,
        };
        assert!(request.validate().is_ok());
    }
}
