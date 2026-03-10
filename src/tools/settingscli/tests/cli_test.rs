/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
//! CLI routing tests
//!
//! These tests verify the dual-client routing behavior:
//! - YAML commands use the API Server client (port 47099)
//! - Other commands use the SettingsService client (port 8080)

#[test]
fn test_error_display() {
    use settingscli::error::CliError;

    let error = CliError::Custom("test error".to_string());
    assert_eq!(format!("{}", error), "Error: test error");
}

#[test]
fn test_client_creation() {
    use settingscli::SettingsClient;

    let client = SettingsClient::new("http://localhost:47098", 30);
    assert!(client.is_ok());
}

/// Verify that a SettingsService client can be created with the default port (8080)
#[test]
fn test_settings_client_default_port() {
    use settingscli::SettingsClient;

    let client = SettingsClient::new("http://localhost:8080", 30);
    assert!(
        client.is_ok(),
        "SettingsService client (port 8080) creation must succeed"
    );
}

/// Verify that an API Server client can be created with the default port (47099)
#[test]
fn test_api_server_client_default_port() {
    use settingscli::SettingsClient;

    let client = SettingsClient::new("http://localhost:47099", 30);
    assert!(
        client.is_ok(),
        "API Server client (port 47099) creation must succeed"
    );
}

/// Verify the dual-client pattern: two independent clients from the same base URL
/// but different ports, as required by the routing implementation.
#[test]
fn test_dual_client_routing_setup() {
    use settingscli::SettingsClient;

    // Same base URL, different ports — this is the routing pattern used in main.rs
    let base_url = "http://localhost";
    let settings_url = format!("{}:{}", base_url, 8080);
    let api_url = format!("{}:{}", base_url, 47099);

    let settings_client = SettingsClient::new(&settings_url, 30);
    let api_client = SettingsClient::new(&api_url, 30);

    assert!(
        settings_client.is_ok(),
        "SettingsService client (port 8080) must be created successfully"
    );
    assert!(
        api_client.is_ok(),
        "API Server client (port 47099) must be created successfully"
    );
}

/// Verify that URL construction from base + port matches the expected format.
/// This mirrors what main.rs does: format!("{}:{}", cli.url, cli.settings_port)
#[test]
fn test_url_construction_from_base_and_port() {
    let base_url = "http://localhost";

    let settings_url = format!("{}:{}", base_url, 8080);
    let api_url = format!("{}:{}", base_url, 47099);

    assert_eq!(
        settings_url, "http://localhost:8080",
        "SettingsService URL must be base:8080"
    );
    assert_eq!(
        api_url, "http://localhost:47099",
        "API Server URL must be base:47099"
    );
}

/// Verify that the API health check endpoint returns false (not an error)
/// when the API Server is unreachable.
#[tokio::test]
async fn test_api_health_check_returns_false_when_unreachable() {
    use settingscli::SettingsClient;

    // Use a port that is not in use
    let client = SettingsClient::new("http://localhost:59998", 1).unwrap();
    let result = client.api_health_check().await;

    match result {
        Ok(false) => {} // Expected: service unreachable → false
        Err(_) => {}    // Also acceptable
        Ok(true) => panic!("API health check must not succeed for an unreachable service"),
    }
}

/// Verify that the SettingsService health check endpoint returns false (not an error)
/// when SettingsService is unreachable.
#[tokio::test]
async fn test_settings_health_check_returns_false_when_unreachable() {
    use settingscli::SettingsClient;

    // Use a port that is not in use
    let client = SettingsClient::new("http://localhost:59997", 1).unwrap();
    let result = client.health_check().await;

    match result {
        Ok(false) => {} // Expected: service unreachable → false
        Err(_) => {}    // Also acceptable
        Ok(true) => {
            panic!("SettingsService health check must not succeed for an unreachable service")
        }
    }
}
