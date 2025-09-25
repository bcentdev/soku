// Ultra Bundler - Modern Architecture
// Entry point with clean separation of concerns

mod utils;
mod core;
mod infrastructure;
mod cli;

use cli::CliHandler;
use utils::UltraError;

#[tokio::main]
async fn main() {
    let handler = CliHandler::new();

    if let Err(e) = handler.run().await {
        // Use enhanced error formatting if available
        match &e {
            UltraError::Parse { .. } | UltraError::Build { .. } => {
                eprintln!("{}", e.format_detailed());
            }
            _ => {
                eprintln!("❌ Error: {}", e);
            }
        }
        std::process::exit(1);
    }
}