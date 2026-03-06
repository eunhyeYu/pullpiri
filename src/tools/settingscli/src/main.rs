/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
//! pirictl - Command Line Interface for Pullpiri
//!
//! This CLI tool provides a convenient way to interact with the Pullpiri API Server
//! and SettingsService via REST APIs. YAML artifact commands communicate directly
//! with the API Server, while other commands use the SettingsService.

use clap::{Parser, Subcommand};
use colored::Colorize;
use pirictl::commands::{apply, board, container, delete, metrics, node, soc};
use pirictl::{Result, SettingsClient};

#[derive(Parser)]
#[command(name = "pirictl")]
#[command(about = "CLI tool for Pullpiri")]
#[command(version)]
#[command(long_about = None)]
struct Cli {
    /// Base URL or host (port is added automatically)
    #[arg(short, long, env = "PICCOLO_URL", default_value = "http://localhost")]
    url: String,

    /// Settings Service port (default: 8080)
    #[arg(long, env = "SETTINGS_PORT")]
    settings_port: Option<u16>,

    /// API Server port (default: 47099)
    #[arg(long, env = "API_PORT")]
    api_port: Option<u16>,

    /// Request timeout in seconds
    #[arg(short, long, default_value = "30")]
    timeout: u64,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Apply a YAML file to the system
    Apply {
        /// Path to YAML file (or '-' for stdin)
        #[arg(short = 'f', long, value_name = "FILE")]
        file: String,
    },

    /// Delete a named resource
    Delete {
        /// Resource type (scenario, package)
        resource_type: String,

        /// Resource name
        name: String,
    },

    /// Get system metrics
    Metrics {
        #[command(subcommand)]
        action: metrics::MetricsAction,
    },

    /// Board-related operations
    Board {
        #[command(subcommand)]
        action: board::BoardAction,
    },

    /// Node-related operations
    Node {
        #[command(subcommand)]
        action: node::NodeAction,
    },

    /// SoC-related operations
    Soc {
        #[command(subcommand)]
        action: soc::SocAction,
    },

    /// Container-related operations
    Container {
        #[command(subcommand)]
        action: container::ContainerAction,
    },

    /// Test connection to SettingsService and API Server
    Health,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Strip any trailing port from the supplied URL so we can append our own ports.
    // Only strips when everything after the final ':' is numeric (i.e. a real port).
    // Scheme colons like "http:" are ignored because "//localhost" is not all-digits.
    // Examples: "http://10.0.0.1:8080" → "http://10.0.0.1"
    //           "http://10.0.0.1"      → "http://10.0.0.1" (unchanged)
    let base_url = {
        let url = cli.url.as_str();
        if let Some(pos) = url.rfind(':') {
            let after = &url[pos + 1..];
            if !after.is_empty() && after.chars().all(|c| c.is_ascii_digit()) {
                &url[..pos]
            } else {
                url
            }
        } else {
            url
        }
    };

    let settings_url = format!("{}:{}", base_url, cli.settings_port.unwrap_or(8080));
    let api_url = format!("{}:{}", base_url, cli.api_port.unwrap_or(47099));

    if cli.verbose {
        println!("{} Settings Service: {}", "ℹ".blue().bold(), settings_url);
        println!("{} API Server: {}", "ℹ".blue().bold(), api_url);
    }

    let settings_client = match SettingsClient::new(&settings_url, cli.timeout) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{} Failed to create settings client: {}",
                "✗".red().bold(),
                e
            );
            std::process::exit(1);
        }
    };

    let api_client = match SettingsClient::new(&api_url, cli.timeout) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} Failed to create API client: {}", "✗".red().bold(), e);
            std::process::exit(1);
        }
    };

    let result = match cli.command {
        // YAML commands — talk directly to the API Server
        Commands::Apply { file } => apply::handle(&api_client, file).await,
        Commands::Delete {
            resource_type,
            name,
        } => delete::handle(&api_client, resource_type, name).await,

        // All other commands — talk to the SettingsService
        Commands::Metrics { action } => metrics::handle(&settings_client, action).await,
        Commands::Board { action } => board::handle(&settings_client, action).await,
        Commands::Node { action } => node::handle(&settings_client, action).await,
        Commands::Soc { action } => soc::handle(&settings_client, action).await,
        Commands::Container { action } => container::handle(&settings_client, action).await,

        Commands::Health => health_check(&settings_client, &api_client).await,
    };

    match result {
        Ok(_) => {
            if cli.verbose {
                println!("{} Command completed successfully", "✓".green().bold());
            }
        }
        Err(e) => {
            eprintln!("{} Command failed: {}", "✗".red().bold(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Perform a health check on both SettingsService and API Server.
/// Returns an error if either service is unreachable.
async fn health_check(settings_client: &SettingsClient, api_client: &SettingsClient) -> Result<()> {
    println!("{} Checking SettingsService health...", "ℹ".blue().bold());
    match settings_client.health_check().await {
        Ok(true) => println!(
            "{} SettingsService is healthy and reachable",
            "✓".green().bold()
        ),
        Ok(false) => {
            println!("{} SettingsService is not reachable", "✗".red().bold());
            return Err(pirictl::error::CliError::Custom(
                "SettingsService health check failed".to_string(),
            ));
        }
        Err(e) => {
            println!(
                "{} SettingsService health check failed: {}",
                "✗".red().bold(),
                e
            );
            return Err(e);
        }
    }

    println!("{} Checking API Server health...", "ℹ".blue().bold());
    match api_client.health_check().await {
        Ok(true) => println!("{} API Server is healthy and reachable", "✓".green().bold()),
        Ok(false) => {
            println!("{} API Server is not reachable", "✗".red().bold());
            return Err(pirictl::error::CliError::Custom(
                "API Server health check failed".to_string(),
            ));
        }
        Err(e) => {
            println!("{} API Server health check failed: {}", "✗".red().bold(), e);
            return Err(e);
        }
    }

    Ok(())
}
