use crate::app::risk::risk_mail_confirmation_service::RiskMailConfirmationService;
use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::Arc;

pub fn confirmation_router(service: Arc<RiskMailConfirmationService>) -> Router {
    Router::new()
        .route("/mail/confirm", get(confirmation_page))
        .route("/api/v1/mail/confirm", post(confirm))
        .with_state(service)
}

async fn confirmation_page() -> Response {
    // GET is read-only: Outlook Safe Links / antivirus previews cannot acknowledge mail.
    ([(header::CACHE_CONTROL,"no-store"),(header::REFERRER_POLICY,"no-referrer"),
        (header::X_CONTENT_TYPE_OPTIONS,"nosniff"),
        (header::CONTENT_SECURITY_POLICY,"default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'")],
        Html(include_str!("risk_mail_confirmation.html"))).into_response()
}

#[derive(Deserialize)]
struct Confirmation {
    token: String,
}

async fn confirm(
    State(service): State<Arc<RiskMailConfirmationService>>,
    Json(request): Json<Confirmation>,
) -> Response {
    let (status, message) = match service.confirm(&request.token).await {
        Ok(true) => (
            StatusCode::OK,
            "已确认收到，本条提醒不会继续补发。请及时关注被监护人的情况。",
        ),
        Ok(false) => (
            StatusCode::BAD_REQUEST,
            "确认链接无效或已过期，请使用邮件中的有效链接。",
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "暂时无法保存确认，请稍后再试。",
        ),
    };
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(serde_json::json!({"message":message})),
    )
        .into_response()
}
