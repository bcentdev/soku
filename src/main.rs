// Soku Bundler - Modern Architecture
// Entry point with clean separation of concerns

mod cli;
mod core;
mod infrastructure;
mod utils;

use cli::CliHandler;
use utils::SokuError;

#[tokio::main]
async fn main() {
    let handler = CliHandler::new();

    if let Err(e) = handler.run().await {
        // Use enhanced error formatting if available
        match &e {
            SokuError::Parse { .. } | SokuError::Build { .. } => {
                eprintln!("{}", e.format_detailed());
            }
            _ => {
                eprintln!("❌ Error: {}", e);
            }
        }
        std::process::exit(1);
    }
}
