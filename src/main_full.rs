mod cache;
mod config;
mod resolver;
// mod graph;  // Temporarily commented due to dependencies
// mod server; // Temporarily commented due to dependencies
mod bundler;
mod plugins;
// mod transform;        // Temporarily commented due to oxc APIs
mod transform_simple;
// mod css;              // Temporarily commented due to lightningcss
mod css_simple;
mod memory;
mod streaming;
mod profiling;

use clap::{Parser, Subcommand};
use anyhow::Result;
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "ultra")]
#[command(about = "Ultra-fast bundler for modern web development")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start development server
    Dev {
        /// Root directory
        #[arg(short, long, default_value = ".")]
        root: String,
        /// Port to serve on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
    /// Build for production
    Build {
        /// Root directory
        #[arg(short, long, default_value = ".")]
        root: String,
        /// Output directory
        #[arg(short, long, default_value = "dist")]
        outdir: String,
    },
    /// Preview production build
    Preview {
        /// Directory to serve
        #[arg(short, long, default_value = "dist")]
        dir: String,
        /// Port to serve on
        #[arg(short, long, default_value_t = 4173)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Dev { root, port } => {
            println!("🚀 Starting Ultra Bundler dev server...");
            println!("   Root: {}", root);
            println!("   Port: {}", port);

            let config = config::Config::load(&root)?;
            let _cache = cache::Cache::new(&config.cache_dir)?;
            let _resolver = resolver::Resolver::new(&config)?;

            // TODO: Uncomment when dependencies are fixed
            // let graph = graph::ModuleGraph::new(cache, resolver);
            // let server = server::DevServer::new(graph, config, port);
            // server.run().await?;

            println!("✅ Ultra Bundler compiled successfully!");
            println!("📝 Note: Full dev server implementation requires fixing oxc/lightningcss dependencies");
        }
        Commands::Build { root, outdir } => {
            println!("🔨 Starting Ultra Bundler build...");
            println!("   Root: {}", root);
            println!("   Output: {}", outdir);

            let config = config::Config::load(&root)?;
            let _cache = cache::Cache::new(&config.cache_dir)?;
            let _resolver = resolver::Resolver::new(&config)?;

            // TODO: Uncomment when dependencies are fixed
            // let graph = graph::ModuleGraph::new(cache, resolver);
            // let bundler = bundler::Bundler::new(graph, config);
            // bundler.build(&outdir).await?;

            println!("✅ Ultra Bundler architecture ready!");
            println!("📝 Note: Full build implementation requires fixing dependencies");
        }
        Commands::Preview { dir, port } => {
            println!("📦 Starting preview server...");
            println!("   Directory: {}", dir);
            println!("   Port: {}", port);

            // TODO: Uncomment when server module is available
            // server::preview_server(dir, port).await?;

            println!("✅ Preview server would start here!");
        }
    }

    Ok(())
}