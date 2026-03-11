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

    // Strip any port that may have been included in the base URL, then attach the correct ports
    let base_host = strip_port(&cli.url);
    let settings_url = format!("{}:{}", base_host, cli.settings_port);
    let api_url = format!("{}:{}", base_host, cli.api_port);

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

    // Execute command – YAML commands go directly to API Server; others go to SettingsService
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

/// Strip any port number from a URL, leaving only scheme + host
fn strip_port(url: &str) -> String {
    // Handle URLs like "http://host:port" or "http://host"
    if let Some(scheme_end) = url.find("://") {
        let scheme = &url[..scheme_end + 3];
        let rest = &url[scheme_end + 3..];
        // Remove port if present (everything after the last ':' that is pure digits)
        if let Some(colon_pos) = rest.rfind(':') {
            let after_colon = &rest[colon_pos + 1..];
            if after_colon.chars().all(|c| c.is_ascii_digit()) {
                return format!("{}{}", scheme, &rest[..colon_pos]);
            }
        }
        url.to_string()
    } else {
        url.to_string()
    }
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
    fn test_strip_port_removes_port() {
        assert_eq!(strip_port("http://localhost:8080"), "http://localhost");
        assert_eq!(strip_port("http://10.0.0.1:47099"), "http://10.0.0.1");
    }

    #[test]
    fn test_strip_port_no_port_unchanged() {
        assert_eq!(strip_port("http://localhost"), "http://localhost");
        assert_eq!(strip_port("http://10.0.0.1"), "http://10.0.0.1");
    }

    #[test]
    fn test_strip_port_builds_correct_urls() {
        let url = "http://10.231.178.2:9999";
        let base = strip_port(url);
        assert_eq!(format!("{}:{}", base, 8080), "http://10.231.178.2:8080");
        assert_eq!(format!("{}:{}", base, 47099), "http://10.231.178.2:47099");
    }
}
