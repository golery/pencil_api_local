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
    println!("Config: {}", config.config_file.display());
    prepare_config(&config.config_file)?;
    serve(config).await.map_err(|err| err.to_string())
}

pub async fn serve(config: Config) -> io::Result<()> {
    let port = config.port;
    let host = config.host.clone();
    let origins = config.cors_origins.join(", ");
    let app = router(config);
    let addr = config::socket_addr(&host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|err| bind_error(port, err))?;
    let bound = listener.local_addr()?.port();
    println!(
        "pencil_api_local listening on http://{}",
        config::socket_addr(&host, bound)
    );
    println!("CORS origins: {origins}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}

fn bind_error(port: u16, err: io::Error) -> io::Error {
    if err.kind() == io::ErrorKind::AddrInUse {
        io::Error::new(
            io::ErrorKind::AddrInUse,
            format!("port {port} is already in use"),
        )
    } else {
        io::Error::new(err.kind(), format!("failed to bind port {port}: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_in_use_names_the_port() {
        let err = bind_error(8558, io::Error::from_raw_os_error(98));
        assert_eq!(err.kind(), io::ErrorKind::AddrInUse);
        assert_eq!(err.to_string(), "port 8558 is already in use");
    }
}

fn prepare_config(path: &Path) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    println!("Config file does not exist: {}", path.display());
    println!("Enter the path to your first book folder to create it.");
    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        return Err(
            "No config file and stdin is not a terminal. Set PENCIL_CONFIG or create the file."
                .to_string(),
        );
    }
    loop {
        eprint!("First book folder: ");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        let mut line = String::new();
        let read = std::io::stdin()
            .read_line(&mut line)
            .map_err(|err| err.to_string())?;
        if read == 0 {
            return Err("No book folder path provided.".to_string());
        }
        let folder = line.trim();
        if folder.is_empty() {
            println!("Enter a folder path.");
            continue;
        }
        match registry::create_with_book(path, folder) {
            Ok(record) => {
                println!("Created {} with book {}", path.display(), record.name);
                return Ok(());
            }
            Err(err) => println!("{err}"),
        }
    }
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
            "--host" => {
                env::set_var("HOST", take_value(&args, &mut index, arg)?);
            }
            _ if arg.starts_with("--port=") => {
                let value = arg["--port=".len()..].to_string();
                if value.is_empty() {
                    return Err("Missing value for --port".to_string());
                }
                env::set_var("PORT", value);
            }
            _ if arg.starts_with("--host=") => {
                let value = arg["--host=".len()..].to_string();
                if value.is_empty() {
                    return Err("Missing value for --host".to_string());
                }
                env::set_var("HOST", value);
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
  -p, --port <port>         Listen port (default: 8558, or PORT)
      --host <addr>         Bind address (default: 127.0.0.1, or HOST)
  -h, --help                Show this help
  -v, --version             Show version

Environment:
  PORT            Listen port
  HOST            Bind address (default: 127.0.0.1). Other machines cannot connect unless this is set.
  PENCIL_CONFIG   Config file (default: ~/.golery/pencil.json)
  CORS_ORIGINS    Extra allowed browser origins, comma-separated

A .env file in the working directory is loaded automatically.
"
);
