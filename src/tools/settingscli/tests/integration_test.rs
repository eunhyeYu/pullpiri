/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
//! Integration tests for pirictl

use pirictl::SettingsClient;

#[tokio::test]
async fn test_client_creation() {
    let client = SettingsClient::new("http://localhost:47099", 30);
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_client_creation_with_invalid_timeout() {
    let client = SettingsClient::new("http://localhost:47099", 0);
    assert!(client.is_ok()); // Client creation should succeed even with 0 timeout
}

#[tokio::test]
async fn test_health_check_with_unreachable_service() {
    // Use a port that's unlikely to be in use
    let client = SettingsClient::new("http://localhost:59999", 1).unwrap();
    let result = client.health_check().await;

    // Should return false or error when service is unreachable
    match result {
        Ok(false) => {} // Expected
        Err(_) => {}    // Also acceptable
        Ok(true) => panic!("Health check should not succeed for unreachable service"),
    }
}

// Note: The following tests require running services.
// They are commented out by default and can be enabled for integration testing.

/*
#[tokio::test]
async fn test_apply_yaml_with_running_api_server() {
    let client = SettingsClient::new("http://localhost:47099", 30).unwrap();
    let yaml = "kind: Scenario\nmetadata:\n  name: test\n";
    let result = client.post_yaml("/api/artifact", yaml).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_scenario_with_running_api_server() {
    let client = SettingsClient::new("http://localhost:47099", 30).unwrap();
    let result = client.delete("/api/v1/scenarios/test-scenario").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_package_with_running_api_server() {
    let client = SettingsClient::new("http://localhost:47099", 30).unwrap();
    let result = client.delete("/api/v1/packages/test-package").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_metrics_endpoint_with_running_settings_service() {
    let client = SettingsClient::new("http://localhost:8080", 30).unwrap();
    let result = client.get("/api/v1/metrics").await;
    assert!(result.is_ok());
}
*/
