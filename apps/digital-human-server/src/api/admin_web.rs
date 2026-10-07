//! Serve the built administrator SPA alongside the existing API.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{any, get},
};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

pub fn mount(api: Router, assets_dir: &str) -> Router {
    if assets_dir.is_empty() {
        return api;
    }
    let dir = Path::new(assets_dir);
    let index = dir.join("index.html");
    if !index.is_file() {
        tracing::warn!(
            directory = assets_dir,
            "管理员页面未构建，继续提供 API 服务"
        );
        return api;
    }

    let spa = Router::new()
        .fallback(get(spa_index))
        .with_state(Arc::new(index));
    let web = Router::new()
        // Unknown API paths must never turn into successful HTML responses.
        .route("/api", any(not_found))
        .route("/api/{*path}", any(not_found))
        .fallback_service(ServeDir::new(dir).fallback(spa))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-cache"),
        ));
    tracing::info!(directory = assets_dir, "Rust 后端已启用管理员页面托管");
    api.merge(web)
}

async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

async fn spa_index(State(index): State<Arc<PathBuf>>, uri: Uri) -> Response {
    let path = uri.path();
    // Missing files and service endpoints keep their actual 404 behavior.
    if path == "/api"
        || path.starts_with("/api/")
        || path == "/assets"
        || path.starts_with("/assets/")
        || path == "/health"
        || path.starts_with("/health/")
        || path.split('/').any(|segment| segment.contains('.'))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    match tokio::fs::read(index.as_ref()).await {
        Ok(html) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            Body::from(html),
        )
            .into_response(),
        Err(error) => {
            tracing::warn!(%error, "管理员页面入口读取失败");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}
