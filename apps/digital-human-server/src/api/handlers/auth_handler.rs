use axum::http::StatusCode;
use axum::{Extension, Json, extract::State, response::IntoResponse};
use chrono::Utc;
use serde::Serialize;
use validator::Validate;

use crate::api::AuthState;
use crate::api::dto::auth_dto::{
    LoginRequest, LogoutRequest, RefreshTokenRequest, RegisterRequest,
};
use crate::api::error::ApiError as AppError;
use crate::app::auth::auth_service::REGISTRATION_PASSWORD_REQUIREMENTS;
use crate::app::auth::auth_service::{AuthTokenPair, AuthenticatedUser, LoginInput};

// ── Response DTOs ────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub id: u64,
    pub username: String,
    pub role: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: &'static str,
    pub user: UserInfo,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub timestamp: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn registration_validation_error(error: validator::ValidationErrors) -> AppError {
    // Validator attaches rejected field values. Never return or log a rejected password.
    if error.field_errors().contains_key("password") {
        AppError::Validation(REGISTRATION_PASSWORD_REQUIREMENTS.into())
    } else {
        AppError::validation(error)
    }
}

fn pair_to_login(pair: AuthTokenPair, username: String) -> LoginResponse {
    LoginResponse {
        user: UserInfo {
            id: pair.user_id,
            username,
            role: pair.role,
        },
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        expires_in: pair.expires_in,
        token_type: pair.token_type,
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn login_challenge(
    State(state): State<AuthState>,
) -> Result<impl IntoResponse, AppError> {
    Ok((
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        Json(state.login_crypto.challenge()?),
    ))
}

pub async fn encrypted_login(
    State(state): State<AuthState>,
    Json(envelope): Json<super::login_crypto::EncryptedLogin>,
) -> Result<impl IntoResponse, AppError> {
    let payload = state.login_crypto.decrypt(envelope)?;
    let response = login(State(state), Json(payload)).await?;
    Ok(([(axum::http::header::CACHE_CONTROL, "no-store")], response))
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "up",
        timestamp: Utc::now().to_rfc3339(),
    })
}

pub async fn register(
    State(state): State<AuthState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate().map_err(registration_validation_error)?;
    let username = payload.username.clone();
    let pair = state
        .auth
        .register(payload.username, payload.password, payload.device_id)
        .await?;
    Ok((StatusCode::CREATED, Json(pair_to_login(pair, username))))
}

pub async fn login(
    State(state): State<AuthState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    payload.validate().map_err(AppError::validation)?;
    let username = payload.username.clone();
    let pair = state
        .auth
        .login(LoginInput {
            username: payload.username,
            password: payload.password,
            device_id: payload.device_id,
        })
        .await?;
    Ok(Json(pair_to_login(pair, username)))
}

pub async fn refresh_token(
    State(state): State<AuthState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshResponse>, AppError> {
    payload.validate().map_err(AppError::validation)?;
    let pair = state
        .auth
        .refresh(&payload.refresh_token, payload.device_id)
        .await?;
    Ok(Json(RefreshResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        expires_in: pair.expires_in,
    }))
}

pub async fn logout(
    State(state): State<AuthState>,
    Json(payload): Json<LogoutRequest>,
) -> Result<StatusCode, AppError> {
    payload.validate().map_err(AppError::validation)?;
    state.auth.logout(&payload.refresh_token).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(Extension(user): Extension<AuthenticatedUser>) -> Json<UserInfo> {
    Json(UserInfo {
        id: user.user_id,
        username: user.username,
        role: user.role.to_string(),
    })
}

#[cfg(test)]
mod registration_tests {
    use super::*;

    #[test]
    fn registration_validation_does_not_disclose_the_rejected_password() {
        let request = RegisterRequest {
            username: "new_user".into(),
            password: "privateweakpassword".into(),
            device_id: None,
        };
        let error = registration_validation_error(request.validate().unwrap_err());
        assert!(matches!(error, AppError::Validation(_)));
        assert!(
            error
                .to_string()
                .contains(REGISTRATION_PASSWORD_REQUIREMENTS)
        );
        assert!(!error.to_string().contains(&request.password));
    }
}
