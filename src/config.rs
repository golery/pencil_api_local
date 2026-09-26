use std::env;
use std::path::{Path, PathBuf};

const DEFAULT_CORS_ORIGINS: &[&str] = &[
    "https://pencil.golery.com",
    "http://localhost:3000",
    "http://127.0.0.1:3000",
];

#[derive(Clone)]
pub struct Config {
    pub books_file: PathBuf,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let port_raw = env::var("PORT").unwrap_or_else(|_| "8300".to_string());
        let port: u16 = port_raw
            .parse()
            .ok()
            .filter(|port| *port > 0)
            .ok_or_else(|| format!("Invalid PORT: {port_raw}"))?;

        let books_raw = env::var("BOOKS_FILE").unwrap_or_else(|_| "./data/books.json".to_string());
        let books_file = resolve_path(&books_raw);

        let mut cors_origins: Vec<String> = DEFAULT_CORS_ORIGINS
            .iter()
            .map(|origin| (*origin).to_string())
            .collect();
        if let Ok(extra) = env::var("CORS_ORIGINS") {
            for origin in extra.split(',') {
                let origin = origin.trim();
                if origin.is_empty() || cors_origins.iter().any(|existing| existing == origin) {
                    continue;
                }
                cors_origins.push(origin.to_string());
            }
        }

        Ok(Self {
            books_file,
            port,
            cors_origins,
        })
    }
}

/// Lexical absolute path, matching Node `path.resolve` (symlinks stay unresolved).
pub fn resolve_path(path: &str) -> PathBuf {
    let input = Path::new(path);
    let combined = if input.is_absolute() {
        input.to_path_buf()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(input)
    };

    let mut out = PathBuf::new();
    for component in combined.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else {
        out
    }
}
