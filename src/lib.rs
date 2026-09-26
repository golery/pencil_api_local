mod config;
mod error;
mod http;
mod ids;
mod registry;
mod scan;

use std::env;
use std::io;
use std::path::Path;

pub use config::Config;
pub use http::router;

pub async fn run() -> Result<(), String> {
    load_dotenv();
    apply_cli(env::args().skip(1))?;
    let config = Config::load()?;
    serve(config).await.map_err(|err| err.to_string())
}

pub async fn serve(config: Config) -> io::Result<()> {
    let port = config.port;
    let books_file = config.books_file.display().to_string();
    let origins = config.cors_origins.join(", ");
    let app = router(config);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    let bound = listener.local_addr()?.port();
    println!("pencil_api_local listening on http://localhost:{bound}");
    println!("BOOKS_FILE={books_file}");
    println!("CORS origins: {origins}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}

fn apply_cli<I>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            "-v" | "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-p" | "--port" => {
                env::set_var("PORT", take_value(&args, &mut index, arg)?);
            }
            "-b" | "--books-file" => {
                env::set_var("BOOKS_FILE", take_value(&args, &mut index, arg)?);
            }
            _ if arg.starts_with("--port=") => {
                let value = arg["--port=".len()..].to_string();
                if value.is_empty() {
                    return Err("Missing value for --port".to_string());
                }
                env::set_var("PORT", value);
            }
            _ if arg.starts_with("--books-file=") => {
                let value = arg["--books-file=".len()..].to_string();
                if value.is_empty() {
                    return Err("Missing value for --books-file".to_string());
                }
                env::set_var("BOOKS_FILE", value);
            }
            _ => return Err(format!("Unknown option: {arg}\n\n{HELP}")),
        }
        index += 1;
    }
    Ok(())
}

fn take_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    *index += 1;
    match args.get(*index) {
        Some(value) if !value.starts_with('-') => Ok(value.clone()),
        _ => Err(format!("Missing value for {flag}")),
    }
}

fn load_dotenv() {
    let Ok(text) = std::fs::read_to_string(Path::new(".env")) else {
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || env::var(key).is_ok() {
            continue;
        }
        let value = value.trim();
        let value = if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };
        env::set_var(key, value);
    }
}

const HELP: &str = concat!(
    "pencil-api-local ",
    env!("CARGO_PKG_VERSION"),
    "

Start the local Pencil vault API.

Usage:
  pencil-api-local [options]

Options:
  -p, --port <port>         Listen port (default: 8300, or PORT)
  -b, --books-file <path>   Book registry JSON (default: ./data/books.json, or BOOKS_FILE)
  -h, --help                Show this help
  -v, --version             Show version

Environment:
  PORT            Listen port
  BOOKS_FILE      Book registry JSON
  CORS_ORIGINS    Extra allowed browser origins, comma-separated

A .env file in the working directory is loaded automatically.
"
);
