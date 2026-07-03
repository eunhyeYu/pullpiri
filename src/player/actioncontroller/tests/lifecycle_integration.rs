/*
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

//! Integration tests for lifecycle gRPC client
//! 
//! These tests require the grpc_lifecycle server to be running.
//! To run the server:
//!   cd /home/lge/Desktop/2track/orchestrator
//!   cargo run -p grpc_lifecycle --bin lifecycle_server
//!
//! Or use the test script:
//!   /home/lge/Desktop/pullpiri/scripts/test_lifecycle_integration.sh

use actioncontroller::grpc::sender::lifecycle;
use actioncontroller::runtime::lifecycle::RestartConfig;

#[tokio::test]
#[ignore] // Requires lifecycle server to be running
async fn test_lifecycle_server_connection() {
    // Test basic connection to lifecycle server
    let result = lifecycle::get_status_by_service("nonexistent").await;
    
    // Should connect successfully even if service doesn't exist
    // Error would indicate server connection issue
    assert!(result.is_ok(), "Failed to connect to lifecycle server: {:?}", result.err());
}

#[tokio::test]
#[ignore] // Requires lifecycle server to be running
async fn test_start_and_stop_binary() {
    let service_name = "test-lifecycle-binary";
    let restart_config = RestartConfig {
        policy: 0, // Never
        max_retries: 0,
        restart_delay_secs: 0,
    };
    
    // Start the binary
    let start_result = lifecycle::start_binary(
        service_name,
        "/bin/sleep",
        vec!["5".to_string()],
        restart_config,
    ).await;
    assert!(start_result.is_ok(), "Failed to start binary: {:?}", start_result.err());
    
    // Small delay to let it start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Check status
    let status_result = lifecycle::get_status_by_service(service_name).await;
    assert!(status_result.is_ok(), "Failed to get status: {:?}", status_result.err());
    
    if let Ok(status) = status_result {
        assert_eq!(status.processes.len(), 1, "Expected 1 process");
        let process = &status.processes[0];
        assert_eq!(process.service_name, service_name);
        assert!(process.pid > 0, "Expected valid PID");
    }
    
    // Stop the binary
    let stop_result = lifecycle::stop_binary_by_service(service_name, false).await;
    assert!(stop_result.is_ok(), "Failed to stop binary: {:?}", stop_result.err());
    
    // Verify it's stopped
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let status_after = lifecycle::get_status_by_service(service_name).await;
    if let Ok(status) = status_after {
        assert_eq!(status.processes.len(), 0, "Process should be stopped");
    }
}

#[tokio::test]
#[ignore] // Requires lifecycle server to be running
async fn test_restart_with_policy() {
    let service_name = "test-restart-binary";
    let restart_config = RestartConfig {
        policy: 1, // OnFailure
        max_retries: 2,
        restart_delay_secs: 0, // Use fast restart for testing
    };
    
    // Start binary that will fail
    let start_result = lifecycle::start_binary(
        service_name,
        "/bin/sh",
        vec!["-c".to_string(), "exit 1".to_string()],
        restart_config,
    ).await;
    assert!(start_result.is_ok(), "Failed to start binary: {:?}", start_result.err());
    
    // Wait for it to crash and be scheduled for restart
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Check status - should be in PendingRestart or restarted
    let status_result = lifecycle::get_status_by_service(service_name).await;
    assert!(status_result.is_ok(), "Failed to get status: {:?}", status_result.err());
    
    if let Ok(status) = status_result {
        // Should either be running again or scheduled for restart
        if !status.processes.is_empty() {
            let process = &status.processes[0];
            assert!(
                process.state.contains("PendingRestart") || process.restart_count > 0,
                "Expected process to be in PendingRestart or restarted state, got: {}",
                process.state
            );
        }
    }
    
    // Cleanup
    let _ = lifecycle::stop_binary_by_service(service_name, true).await;
}

#[tokio::test]
#[ignore] // Requires lifecycle server to be running
async fn test_error_handling() {
    let service_name = "test-error-binary";
    let restart_config = RestartConfig {
        policy: 0, // Never
        max_retries: 0,
        restart_delay_secs: 0,
    };
    
    // Test with non-existent binary path
    let result = lifecycle::start_binary(
        service_name,
        "/nonexistent/binary",
        vec![],
        restart_config,
    ).await;
    
    // Should fail with meaningful error
    assert!(result.is_err(), "Expected error for non-existent binary");
    
    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("not found") || error_msg.contains("Lifecycle start_binary failed"),
            "Error message should indicate file not found: {}",
            error_msg
        );
    }
}

#[tokio::test]
#[ignore] // Requires lifecycle server to be running
async fn test_stop_all() {
    // Start multiple binaries
    let restart_config = RestartConfig {
        policy: 0,
        max_retries: 0,
        restart_delay_secs: 0,
    };
    
    for i in 1..=3 {
        let service_name = format!("test-stopall-{}", i);
        let _ = lifecycle::start_binary(
            &service_name,
            "/bin/sleep",
            vec!["10".to_string()],
            restart_config,
        ).await;
    }
    
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    // Stop all
    let result = lifecycle::stop_all_binaries(true).await;
    assert!(result.is_ok(), "Failed to stop all: {:?}", result.err());
    
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    // Verify all are stopped
    for i in 1..=3 {
        let service_name = format!("test-stopall-{}", i);
        let status = lifecycle::get_status_by_service(&service_name).await;
        if let Ok(status) = status {
            assert_eq!(status.processes.len(), 0, "Process {} should be stopped", service_name);
        }
    }
}
