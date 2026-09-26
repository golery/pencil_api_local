use std::fs;
use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use pencil_api_local::{router, Config};
use serde_json::Value;
use tower::ServiceExt;

fn test_config(books_file: PathBuf) -> Config {
    Config {
        config_file: books_file,
        host: "127.0.0.1".to_string(),
        port: 8558,
        cors_origins: vec![
            "https://pencil.golery.com".to_string(),
            "http://localhost:3000".to_string(),
            "http://127.0.0.1:3000".to_string(),
        ],
    }
}

fn ensure_host(request: Request<Body>) -> Request<Body> {
    if request.headers().contains_key("host") {
        return request;
    }
    let (mut parts, body) = request.into_parts();
    parts.headers.insert(
        axum::http::header::HOST,
        axum::http::HeaderValue::from_static("localhost:8558"),
    );
    Request::from_parts(parts, body)
}

async fn send(
    app: axum::Router,
    request: Request<Body>,
) -> (StatusCode, Value, axum::http::HeaderMap) {
    let response = app.oneshot(ensure_host(request)).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, json, headers)
}

#[tokio::test]
async fn health_signin_user_and_cors() {
    let app = router(test_config(std::env::temp_dir().join("unused-books.json")));

    let (status, json, _) = send(
        app.clone(),
        Request::builder()
            .uri("/api/health")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::json!({ "ok": true }));

    let (status, json, headers) = send(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/public/signin")
            .header("Origin", "http://localhost:3000")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::json!({ "token": "local" }));
    assert_eq!(
        headers["access-control-allow-origin"],
        "http://localhost:3000"
    );
    assert_eq!(headers["access-control-allow-private-network"], "true");

    let (status, json, _) = send(
        app.clone(),
        Request::builder()
            .uri("/api/user")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        json,
        serde_json::json!({ "id": 1, "email": "local@pencil" })
    );

    let (status, _, headers) = send(
        app,
        Request::builder()
            .method("OPTIONS")
            .uri("/api/pencil/book")
            .header("Origin", "https://pencil.golery.com")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        headers["access-control-allow-origin"],
        "https://pencil.golery.com"
    );
}

#[tokio::test]
async fn rejects_foreign_origin_host_and_content_type() {
    let app = router(test_config(std::env::temp_dir().join("unused-books.json")));

    let (status, json, headers) = send(
        app.clone(),
        Request::builder()
            .uri("/api/health")
            .header("Origin", "https://evil.test")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["error"], "Origin is not allowed");
    assert!(headers.get("access-control-allow-origin").is_none());

    let (status, json, headers) = send(
        app.clone(),
        Request::builder()
            .uri("/api/health")
            .header("Origin", "http://localhost:3000")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::json!({ "ok": true }));
    assert_eq!(
        headers["access-control-allow-origin"],
        "http://localhost:3000"
    );

    let (status, _, _) = send(
        app.clone(),
        Request::builder()
            .uri("/api/health")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, json, _) = send(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/pencil/book")
            .header("content-type", "text/plain")
            .body(Body::from(r#"{"name":"x","path":"/"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(json["error"], "Content-Type must be application/json");

    let (status, json, _) = send(
        app.clone(),
        Request::builder()
            .uri("/api/health")
            .header("host", "evil.com:8558")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["error"], "Host is not allowed");

    let (status, json, _) = send(
        app.clone(),
        Request::builder()
            .uri("/api/health")
            .header("host", "localhost:8558")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::json!({ "ok": true }));

    let (status, json, _) = send(
        app.clone(),
        Request::builder()
            .uri("/api/health")
            .header("host", "127.0.0.1:8558")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::json!({ "ok": true }));

    let (status, _, headers) = send(
        app,
        Request::builder()
            .method("OPTIONS")
            .uri("/api/pencil/book")
            .header("Origin", "https://evil.test")
            .header("Access-Control-Request-Method", "POST")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(headers.get("access-control-allow-origin").is_none());
}

#[tokio::test]
async fn allows_an_opt_in_bind_host() {
    let mut config = test_config(std::env::temp_dir().join("unused-books.json"));
    config.host = "192.168.1.10".to_string();
    let app = router(config);

    let (status, _, _) = send(
        app.clone(),
        Request::builder()
            .uri("/api/health")
            .header("host", "192.168.1.10:8558")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, _) = send(
        app,
        Request::builder()
            .uri("/api/health")
            .header("host", "10.0.0.8:8558")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn links_scans_renames_writes_and_unlinks() {
    let dir = std::env::temp_dir().join(format!("pencil-api-local-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Notes")).unwrap();
    fs::write(dir.join("Notes/Welcome.md"), "# Hello\n").unwrap();
    fs::create_dir_all(dir.join("Notes/Projects")).unwrap();
    fs::write(dir.join("Notes/Projects/Overview.md"), "overview\n").unwrap();
    fs::write(dir.join("Notes/.secret.md"), "hidden\n").unwrap();
    let books = dir.join("books.json");
    let app = router(test_config(books));
    let folder = dir.join("Notes");

    let (status, created, _) = send(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/pencil/book")
            .header("content-type", "application/json")
            .body(Body::from(format!(
                "{{\"name\":\"Personal\",\"path\":\"{}\"}}",
                folder.display()
            )))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let book_id = created["book"]["id"].as_u64().unwrap();
    assert_eq!(created["book"]["name"], "Personal");
    assert_eq!(created["book"]["userId"], "local");
    assert!(created["book"]["rootId"].as_u64().unwrap() > 0);

    let (status, books, _) = send(
        app.clone(),
        Request::builder()
            .uri("/api/pencil/book")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(books.as_array().unwrap().len(), 1);

    let (status, nodes, _) = send(
        app.clone(),
        Request::builder()
            .uri(format!("/api/pencil/book/{book_id}/node"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let nodes = nodes.as_array().unwrap();
    let welcome = nodes.iter().find(|node| node["name"] == "Welcome").unwrap();
    assert_eq!(welcome["text"], "# Hello\n");
    assert_eq!(welcome["relPath"], "Welcome.md");
    assert!(welcome["createTime"].is_null());
    assert!(nodes.iter().all(|node| node["name"] != ".secret"));
    let root = nodes
        .iter()
        .find(|node| node["parentId"].is_null())
        .unwrap();
    assert_eq!(root["children"].as_array().unwrap().len(), 2);

    let welcome_id = welcome["id"].as_u64().unwrap();
    let (status, written, _) = send(
        app.clone(),
        Request::builder()
            .method("PUT")
            .uri(format!("/api/pencil/book/{book_id}/node/{welcome_id}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"text":"updated"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(written["text"], "updated");
    assert_eq!(
        fs::read_to_string(folder.join("Welcome.md")).unwrap(),
        "updated"
    );

    let (status, renamed, _) = send(
        app.clone(),
        Request::builder()
            .method("PATCH")
            .uri(format!("/api/pencil/book/{book_id}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"Renamed"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(renamed["name"], "Renamed");

    let (status, duplicate, _) = send(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/pencil/book")
            .header("content-type", "application/json")
            .body(Body::from(format!(
                "{{\"name\":\"Again\",\"path\":\"{}\"}}",
                folder.display()
            )))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(duplicate["error"], "Folder is already linked");

    let (status, missing, _) = send(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/pencil/book/reorder")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(
        missing["error"],
        "Write operations are not supported by pencil_api_local"
    );

    let (status, _, _) = send(
        app.clone(),
        Request::builder()
            .method("DELETE")
            .uri(format!("/api/pencil/book/{book_id}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, json, _) = send(
        app,
        Request::builder()
            .uri("/api/pencil/book")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::json!([]));

    let _ = fs::remove_dir_all(&dir);
}
