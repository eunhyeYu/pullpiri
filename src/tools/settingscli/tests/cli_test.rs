/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
//! CLI-specific tests

#[test]
fn test_error_display() {
    use pirictl::error::CliError;

    let error = CliError::Custom("test error".to_string());
    assert_eq!(format!("{}", error), "Error: test error");
}

#[test]
fn test_invalid_resource_type_display() {
    use pirictl::error::CliError;

    let error = CliError::InvalidResourceType("unknown".to_string());
    let msg = format!("{}", error);
    assert!(msg.contains("unknown"));
    assert!(msg.contains("scenario"));
    assert!(msg.contains("package"));
}

#[test]
fn test_client_creation() {
    use pirictl::SettingsClient;

    let client = SettingsClient::new("http://localhost:47099", 30);
    assert!(client.is_ok());
}
