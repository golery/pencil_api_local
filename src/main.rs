#[tokio::main]
async fn main() {
    if let Err(err) = pencil_api_local::run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
