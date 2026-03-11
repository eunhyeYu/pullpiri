/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
//! SettingsCLI - Command Line Interface for Pullpiri SettingsService
//!
//! This CLI tool provides a convenient way to interact with the Pullpiri SettingsService
//! via REST APIs. It supports various operations for managing boards, nodes, and SoCs.

use clap::{Parser, Subcommand};
use colored::Colorize;
use settingscli::commands::{board, container, metrics, node, soc, yaml};
use settingscli::{Result, SettingsClient};
use url::Url;

#[derive(Parser)]
#[command(name = "settingscli")]
#[command(about = "CLI tool for Pullpiri SettingsService")]
#[command(version)]
#[command(long_about = None)]
struct Cli {
    /// Base URL (host only, without port)
    #[arg(short, long, env = "PICCOLO_URL", default_value = "http://localhost")]
    url: String,

    /// SettingsService port
    #[arg(long, env = "SETTINGS_PORT", default_value = "8080")]
    settings_port: u16,

    /// API Server port
    #[arg(long, env = "API_PORT", default_value = "47099")]
    api_port: u16,

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
    /// YAML artifact management
    Yaml {
        #[command(subcommand)]
        action: yaml::YamlAction,
    },
    /// Test connection to SettingsService
    Health,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Use the `url` crate to parse the base URL, replace the port, and build the final URLs
    let settings_url = build_url_with_port(&cli.url, cli.settings_port).unwrap_or_else(|e| {
        eprintln!("{} Invalid URL: {}", "✗".red().bold(), e);
        std::process::exit(1);
    });
    let api_url = build_url_with_port(&cli.url, cli.api_port).unwrap_or_else(|e| {
        eprintln!("{} Invalid URL: {}", "✗".red().bold(), e);
        std::process::exit(1);
    });

    if cli.verbose {
        println!(
            "{} SettingsService URL: {}",
            "ℹ".blue().bold(),
            settings_url
        );
        println!("{} API Server URL: {}", "ℹ".blue().bold(), api_url);
    }

    // Create two clients: one for SettingsService, one for API Server
    let settings_client = match SettingsClient::new(&settings_url, cli.timeout) {
        Ok(client) => client,
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
        Ok(client) => client,
        Err(e) => {
            eprintln!("{} Failed to create API client: {}", "✗".red().bold(), e);
            std::process::exit(1);
        }
    };

    // Execute command - YAML commands go directly to API Server; others go to SettingsService
    let result = match cli.command {
        Commands::Metrics { action } => metrics::handle(&settings_client, action).await,
        Commands::Board { action } => board::handle(&settings_client, action).await,
        Commands::Node { action } => node::handle(&settings_client, action).await,
        Commands::Soc { action } => soc::handle(&settings_client, action).await,
        Commands::Container { action } => container::handle(&settings_client, action).await,
        Commands::Yaml { action } => yaml::handle(&api_client, action).await,
        Commands::Health => health_check(&settings_client).await,
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

/// Build a URL by parsing the given base URL and replacing (or setting) the port.
///
/// Any port already present in `base_url` is overwritten with `port`.
fn build_url_with_port(base_url: &str, port: u16) -> std::result::Result<String, String> {
    let mut parsed = Url::parse(base_url).map_err(|e| format!("'{}': {}", base_url, e))?;
    parsed
        .set_port(Some(port))
        .map_err(|_| format!("cannot set port on '{}'", base_url))?;
    // Remove trailing slash for consistency with the rest of the codebase
    Ok(parsed.as_str().trim_end_matches('/').to_string())
}

/// Perform a health check on the SettingsService
async fn health_check(client: &SettingsClient) -> Result<()> {
    println!("{} Checking SettingsService health...", "ℹ".blue().bold());

    match client.health_check().await {
        Ok(true) => {
            println!(
                "{} SettingsService is healthy and reachable",
                "✓".green().bold()
            );
        }
        Ok(false) => {
            println!("{} SettingsService is not reachable", "✗".red().bold());
            return Err(settingscli::error::CliError::Custom(
                "Health check failed".to_string(),
            ));
        }
        Err(e) => {
            println!("{} Health check failed: {}", "✗".red().bold(), e);
            return Err(e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url_replaces_port() {
        assert_eq!(
            build_url_with_port("http://localhost:8080", 47099).unwrap(),
            "http://localhost:47099"
        );
        assert_eq!(
            build_url_with_port("http://10.0.0.1:9999", 8080).unwrap(),
            "http://10.0.0.1:8080"
        );
    }

    #[test]
    fn test_build_url_sets_port_when_absent() {
        assert_eq!(
            build_url_with_port("http://localhost", 8080).unwrap(),
            "http://localhost:8080"
        );
        assert_eq!(
            build_url_with_port("http://10.231.178.2", 47099).unwrap(),
            "http://10.231.178.2:47099"
        );
    }

    #[test]
    fn test_build_url_invalid_input() {
        assert!(build_url_with_port("not-a-url", 8080).is_err());
    }
}
