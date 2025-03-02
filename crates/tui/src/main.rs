use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use dotenv::dotenv;
use log::info;
use anyhow::anyhow;

use crate::app::App;

mod action;
mod app;
mod cli;
mod components;
mod config;
mod errors;
mod logging;
mod tui;

/// Show logging location to user
fn show_log_location() -> std::result::Result<(), anyhow::Error> {
    // Get the data directory from config
    let directory = crate::config::get_data_dir();
    let log_path = directory.join(crate::logging::LOG_FILE.clone());
    
    // Display notice about log location
    eprintln!("Logs will be written to: {}", log_path.display());
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment variables from .env file
    dotenv().ok();
    
    // Initialize color_eyre for error handling
    crate::errors::init()?;
    
    // Initialize logging
    crate::logging::init()?;
    
    // Show log file location
    if let Err(e) = show_log_location() {
        eprintln!("Warning: Could not determine log location: {}", e);
    }
    
    info!("Starting The Brain...");
    
    // Log system information
    info!("System Information:");
    info!("- OS: {}", std::env::consts::OS);
    info!("- Architecture: {}", std::env::consts::ARCH);
    info!("- Date: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    
    // Log configuration from environment
    if let Ok(log_level) = std::env::var("RUST_LOG") {
        info!("- Log level: {}", log_level);
    } else {
        info!("- Log level: info (default)");
    }
    
    // Log API configuration status
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        info!("- Anthropic API: Configured");
    } else {
        info!("- Anthropic API: Not configured (will use fallback templates)");
    }
    
    // Check for config file
    let config_file = std::path::Path::new("config.toml");
    if config_file.exists() {
        info!("- Config file: Found at {}", config_file.display());
    } else {
        info!("- Config file: Not found, using defaults");
    }
    
    // Parse command line arguments
    let args = Cli::parse();
    
    // Create and run the TUI
    info!("Initializing TUI...");
    let mut app = App::new(args.tick_rate, args.frame_rate)?;
    app.run().await?;
    
    info!("Application shutdown complete");
    Ok(())
}
