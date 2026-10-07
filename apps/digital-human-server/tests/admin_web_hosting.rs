use axum::{Router, routing::get};
use digital_human_server::api::admin_web;
use reqwest::StatusCode;

#[tokio::test]
async fn serves_spa_without_masking_api_or_missing_assets() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("assets")).unwrap();
    std::fs::write(dir.path().join("index.html"), "<html>admin fixture</html>").unwrap();
    std::fs::write(dir.path().join("assets/app.js"), "console.log('fixture')").unwrap();
    // Even if an API-looking file exists, it cannot shadow API 404s.
    std::fs::create_dir(dir.path().join("api")).unwrap();
    std::fs::write(dir.path().join("api/unknown"), "must not be served").unwrap();
    let api = Router::new()
        .route("/health", get(|| async { "healthy" }))
        .route("/api/v1/test", get(|| async { "api response" }));
    let app = admin_web::mount(api, dir.path().to_str().unwrap());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();

    for path in [
        "/",
        "/login",
        "/users",
        "/risks/37",
        "/music",
        "/unknown-page",
    ] {
        let response = client.get(format!("{base}{path}")).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
        assert_eq!(response.headers()["cache-control"], "no-cache");
        assert_eq!(response.text().await.unwrap(), "<html>admin fixture</html>");
    }
    let js = client
        .get(format!("{base}/assets/app.js"))
        .send()
        .await
        .unwrap();
    assert_eq!(js.status(), StatusCode::OK);
    assert!(
        js.headers()["content-type"]
            .to_str()
            .unwrap()
            .contains("javascript")
    );
    for (path, body) in [("/health", "healthy"), ("/api/v1/test", "api response")] {
        assert_eq!(
            client
                .get(format!("{base}{path}"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            body
        );
    }
    for path in [
        "/api",
        "/api/",
        "/api/unknown",
        "/api/v1/missing",
        "/assets/missing.js",
        "/assets/missing",
        "/missing.css",
        "/health/missing",
    ] {
        assert_eq!(
            client
                .get(format!("{base}{path}"))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    assert_eq!(
        client
            .post(format!("{base}/users"))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    let head = client
        .head(format!("{base}/risks/37"))
        .send()
        .await
        .unwrap();
    assert_eq!(head.status(), StatusCode::OK);
    assert!(head.bytes().await.unwrap().is_empty());
    server.abort();
}

#[tokio::test]
async fn unbuilt_or_disabled_admin_does_not_break_api() {
    let dir = tempfile::tempdir().unwrap();
    for path in ["", dir.path().to_str().unwrap()] {
        let app = admin_web::mount(
            Router::new().route("/health", get(|| async { "healthy" })),
            path,
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = reqwest::Client::new();
        assert_eq!(
            client
                .get(format!("{base}/health"))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            client
                .get(format!("{base}/"))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        server.abort();
    }
}
