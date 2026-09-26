use std::env;
use std::path::{Path, PathBuf};

const DEFAULT_CORS_ORIGINS: &[&str] = &[
    "https://pencil.golery.com",
    "http://localhost:3000",
    "http://127.0.0.1:3000",
];

#[derive(Clone)]
pub struct Config {
    pub config_file: PathBuf,
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let port_raw = env::var("PORT").unwrap_or_else(|_| "8558".to_string());
        let port: u16 = port_raw
            .parse()
            .ok()
            .filter(|port| *port > 0)
            .ok_or_else(|| format!("Invalid PORT: {port_raw}"))?;

        let host = listen_host(env::var("HOST").ok().as_deref())?;
        let config_file = config_path()?;

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
            config_file,
            host,
            port,
            cors_origins,
        })
    }
}

/// Bind address from `HOST`. Unset means `127.0.0.1`.
pub fn listen_host(value: Option<&str>) -> Result<String, String> {
    let host = value.unwrap_or("127.0.0.1").trim();
    if host.is_empty() {
        return Err("Invalid HOST: empty".to_string());
    }
    Ok(host.to_string())
}

/// `host:port`, with brackets when `host` is an IPv6 address.
pub fn socket_addr(host: &str, port: u16) -> String {
    let host = host.trim().trim_matches(|ch| ch == '[' || ch == ']');
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

/// `PENCIL_CONFIG` when set, otherwise `~/.golery/pencil.json`.
pub fn config_path() -> Result<PathBuf, String> {
    let configured = env::var("PENCIL_CONFIG").ok();
    let home = env::var("HOME").ok();
    config_path_from(configured.as_deref(), home.as_deref())
}

fn config_path_from(pencil_config: Option<&str>, home: Option<&str>) -> Result<PathBuf, String> {
    if let Some(raw) = pencil_config.map(str::trim).filter(|raw| !raw.is_empty()) {
        return Ok(resolve_path(&expand_tilde_with(raw, home)));
    }
    let home = home
        .map(str::trim)
        .filter(|home| !home.is_empty())
        .ok_or("HOME is not set. Set PENCIL_CONFIG to a config file path.")?;
    Ok(PathBuf::from(home).join(".golery/pencil.json"))
}

fn expand_tilde_with(path: &str, home: Option<&str>) -> String {
    let Some(rest) = path.strip_prefix('~') else {
        return path.to_string();
    };
    let Some(home) = home.filter(|home| !home.is_empty()) else {
        return path.to_string();
    };
    if rest.is_empty() {
        return home.to_string();
    }
    if let Some(rest) = rest.strip_prefix('/') {
        return PathBuf::from(home)
            .join(rest)
            .to_string_lossy()
            .into_owned();
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pencil_config_overrides_the_home_default() {
        let chosen = config_path_from(Some("/tmp/pencil.json"), Some("/home/hly")).unwrap();
        assert_eq!(chosen, PathBuf::from("/tmp/pencil.json"));

        let fallback = config_path_from(None, Some("/home/hly")).unwrap();
        assert_eq!(fallback, PathBuf::from("/home/hly/.golery/pencil.json"));

        let tilde = config_path_from(Some("~/notes/pencil.json"), Some("/home/hly")).unwrap();
        assert_eq!(tilde, PathBuf::from("/home/hly/notes/pencil.json"));
    }

    #[test]
    fn listen_host_defaults_to_loopback() {
        assert_eq!(listen_host(None).unwrap(), "127.0.0.1");
        assert_eq!(listen_host(Some(" 192.168.1.10 ")).unwrap(), "192.168.1.10");
        assert!(listen_host(Some("  ")).is_err());
        assert_eq!(socket_addr("127.0.0.1", 8558), "127.0.0.1:8558");
        assert_eq!(socket_addr("::1", 8558), "[::1]:8558");
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
