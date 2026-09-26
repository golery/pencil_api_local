use std::sync::{Arc, Mutex};

use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::Router;
use axum::Json;
use serde_json::{Map, Value};

use crate::config::Config;
use crate::error::ApiError;
use crate::registry::{self, BookRecord};
use crate::scan::{self, Book};

const NOT_IMPLEMENTED: &str = "Write operations are not supported by pencil_api_local";
const MAX_BODY: usize = 20 * 1024 * 1024;

pub struct AppState {
    pub config: Config,
    registry: Mutex<()>,
}

pub fn router(config: Config) -> Router {
    let state = Arc::new(AppState {
        config,
        registry: Mutex::new(()),
    });
    Router::new().fallback(handle).with_state(state)
}

struct Reply {
    status: StatusCode,
    body: Value,
}

async fn handle(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let origin = header_string(req.headers(), header::ORIGIN);
    if req.method() == Method::OPTIONS {
        return with_cors(
            StatusCode::NO_CONTENT.into_response(),
            &origin,
            &state.config,
        );
    }

    let config = state.config.clone();
    let reply = match dispatch(&state, req).await {
        Ok(reply) => reply,
        Err(err) => {
            eprintln!("{err}");
            Reply {
                status: StatusCode::from_u16(err.status)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                body: serde_json::json!({ "error": err.message }),
            }
        }
    };
    with_cors(
        (reply.status, Json(reply.body)).into_response(),
        &origin,
        &config,
    )
}

async fn dispatch(state: &AppState, req: Request) -> Result<Reply, ApiError> {
    let method = req.method().clone();
    let path = normalize_path(req.uri().path());
    let body = read_json(req).await?;

    if path == "/api/health" && method == Method::GET {
        return Ok(ok_json(serde_json::json!({ "ok": true })));
    }
    if path == "/api/public/signin" && method == Method::POST {
        return Ok(ok_json(serde_json::json!({ "token": "local" })));
    }
    if path == "/api/user" && method == Method::GET {
        return Ok(ok_json(
            serde_json::json!({ "id": 1, "email": "local@pencil" }),
        ));
    }

    let _registry = state.registry.lock().unwrap_or_else(|err| err.into_inner());
    let books_file = &state.config.config_file;

    if path == "/api/pencil/book" && method == Method::GET {
        let records = registry::load_registry(books_file)?;
        let books = scan::list_books_from_registry(&records);
        return Ok(ok_json(serde_json::to_value(books).map_err(json_err)?));
    }

    if path == "/api/pencil/book" && method == Method::POST {
        let name = string_field(&body, "name");
        let folder = string_field(&body, "path");
        let (Some(name), Some(folder)) = (name, folder) else {
            return Ok(err_json(
                StatusCode::BAD_REQUEST,
                "name and path are required",
            ));
        };
        let abs = crate::config::resolve_path(&folder);
        let meta = std::fs::metadata(&abs).ok();
        if meta.as_ref().is_none_or(|meta| !meta.is_dir()) {
            return Ok(err_json(
                StatusCode::BAD_REQUEST,
                format!("Not a directory: {}", abs.display()),
            ));
        }
        let record = registry::add_book(books_file, &name, &folder)?;
        let (book, _) = scan::scan_book_folder(&record)?;
        return Ok(Reply {
            status: StatusCode::CREATED,
            body: serde_json::json!({ "book": book }),
        });
    }

    if let Some(book_id) = book_id_from_path(&path) {
        if method == Method::PATCH {
            let Some(name) = string_field(&body, "name") else {
                return Ok(err_json(StatusCode::BAD_REQUEST, "name is required"));
            };
            let record = registry::update_book_name(books_file, book_id, &name)?;
            let book = match scan::scan_book_folder(&record) {
                Ok((book, _)) => book,
                Err(_) => fallback_book(&record),
            };
            return Ok(ok_json(serde_json::to_value(book).map_err(json_err)?));
        }
        if method == Method::DELETE {
            registry::remove_book(books_file, book_id)?;
            return Ok(ok_json(serde_json::json!({ "ok": true })));
        }
    }

    if let Some(book_id) = book_nodes_from_path(&path) {
        if method == Method::GET {
            let record = require_book(books_file, book_id)?;
            let (_, nodes) = scan::scan_book_folder(&record)?;
            return Ok(ok_json(serde_json::to_value(nodes).map_err(json_err)?));
        }
    }

    if let Some((book_id, node_id)) = write_node_from_path(&path) {
        if method == Method::PUT {
            let Some(text) = body.get("text").and_then(Value::as_str) else {
                return Ok(err_json(
                    StatusCode::BAD_REQUEST,
                    "text (string) is required",
                ));
            };
            let record = require_book(books_file, book_id)?;
            let node = scan::write_node_text(&record, node_id, text)?;
            return Ok(ok_json(serde_json::to_value(node).map_err(json_err)?));
        }
    }

    if method != Method::GET && is_unimplemented_mutation(&path) {
        return Ok(err_json(StatusCode::NOT_IMPLEMENTED, NOT_IMPLEMENTED));
    }

    Ok(err_json(
        StatusCode::NOT_FOUND,
        format!("Not found: {} {path}", method.as_str()),
    ))
}

fn require_book(books_file: &std::path::Path, book_id: u32) -> Result<BookRecord, ApiError> {
    registry::get_book_record(books_file, book_id)?
        .ok_or_else(|| ApiError::new(404, "Book not found"))
}

fn fallback_book(record: &BookRecord) -> Book {
    Book {
        id: record.id,
        code: record.name.clone(),
        root_id: 0,
        name: record.name.clone(),
        order: record.order,
        user_id: "local".to_string(),
        folder_path: record.path.clone(),
    }
}

fn ok_json(body: Value) -> Reply {
    Reply {
        status: StatusCode::OK,
        body,
    }
}

fn err_json(status: StatusCode, message: impl Into<String>) -> Reply {
    Reply {
        status,
        body: serde_json::json!({ "error": message.into() }),
    }
}

fn json_err(err: serde_json::Error) -> ApiError {
    ApiError::new(500, err.to_string())
}

async fn read_json(req: Request) -> Result<Map<String, Value>, ApiError> {
    let bytes = axum::body::to_bytes(req.into_body(), MAX_BODY)
        .await
        .map_err(|_| ApiError::new(400, "request body too large"))?;
    if bytes.is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(map)) => Ok(map),
        _ => Ok(Map::new()),
    }
}

fn string_field(body: &Map<String, Value>, key: &str) -> Option<String> {
    let value = body.get(key)?.as_str()?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

fn book_id_from_path(path: &str) -> Option<u32> {
    let rest = path.strip_prefix("/api/pencil/book/")?;
    if rest.contains('/') {
        None
    } else {
        parse_u32(rest)
    }
}

fn book_nodes_from_path(path: &str) -> Option<u32> {
    let rest = path.strip_prefix("/api/pencil/book/")?;
    let (id, tail) = rest.split_once('/')?;
    if tail == "node" {
        parse_u32(id)
    } else {
        None
    }
}

fn write_node_from_path(path: &str) -> Option<(u32, u32)> {
    let rest = path.strip_prefix("/api/pencil/book/")?;
    let mut parts = rest.split('/');
    let book = parts.next()?;
    let node_word = parts.next()?;
    let node = parts.next()?;
    if parts.next().is_some() || node_word != "node" {
        return None;
    }
    Some((parse_u32(book)?, parse_u32(node)?))
}

fn is_unimplemented_mutation(path: &str) -> bool {
    if path == "/api/pencil/book/reorder" || path == "/api/pencil/update" {
        return true;
    }
    let Some(rest) = path.strip_prefix("/api/pencil/") else {
        return false;
    };
    let Some((kind, id)) = rest.split_once('/') else {
        return false;
    };
    matches!(kind, "move" | "add" | "delete") && parse_u32(id).is_some()
}

fn parse_u32(value: &str) -> Option<u32> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn header_string(headers: &HeaderMap, name: header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn with_cors(mut response: Response, origin: &Option<String>, config: &Config) -> Response {
    let headers = response.headers_mut();
    insert(
        headers,
        "Access-Control-Allow-Methods",
        "GET, POST, PUT, PATCH, DELETE, OPTIONS",
    );
    insert(
        headers,
        "Access-Control-Allow-Headers",
        "Authorization, Content-Type, appId",
    );
    insert(headers, "Access-Control-Max-Age", "86400");
    insert(headers, "Access-Control-Allow-Private-Network", "true");
    if let Some(origin) = origin {
        if config.cors_origins.iter().any(|allowed| allowed == origin) {
            insert(headers, "Access-Control-Allow-Origin", origin);
            insert(headers, "Access-Control-Allow-Credentials", "true");
            insert(headers, header::VARY.as_str(), "Origin");
        }
    }
    response
}

fn insert(headers: &mut HeaderMap, name: &str, value: &str) {
    if let (Ok(name), Ok(value)) = (
        name.parse::<axum::http::HeaderName>(),
        HeaderValue::from_str(value),
    ) {
        headers.insert(name, value);
    }
}
