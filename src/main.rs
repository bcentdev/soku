// Ultra Bundler - Modern Architecture
// Entry point with clean separation of concerns

mod utils;
mod core;
mod infrastructure;
mod cli;

use cli::CliHandler;

#[tokio::main]
async fn main() {
    let handler = CliHandler::new();

    if let Err(e) = handler.run().await {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}