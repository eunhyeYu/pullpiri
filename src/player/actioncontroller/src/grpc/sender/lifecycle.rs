/*
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::Result;
use tonic::Request;
use crate::runtime::lifecycle::RestartConfig;

pub mod lifecycle {
    tonic::include_proto!("lifecycle");
}

use lifecycle::binary_lifecycle_client::BinaryLifecycleClient;

fn lifecycle_endpoint() -> String {
    std::env::var("LIFECYCLE_GRPC_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

pub async fn start_binary(
    service_name: &str,
    binary_path: &str,
    args: Vec<String>,
    restart_config: RestartConfig,
) -> Result<()> {
    let endpoint = lifecycle_endpoint();
    let mut client = BinaryLifecycleClient::connect(endpoint.clone())
        .await
        .map_err(|e| format!("Failed to connect to lifecycle service at {}: {}", endpoint, e))?;

    let request = lifecycle::StartRequest {
        service_name: service_name.to_string(),
        binary_path: binary_path.to_string(),
        args: args.clone(),
        restart_policy: restart_config.policy,
        max_retries: restart_config.max_retries,
        restart_delay_secs: restart_config.restart_delay_secs,
    };

    let response = client
        .start_binary(Request::new(request))
        .await
        .map_err(|e| format!("gRPC start_binary failed for service '{}': {}", service_name, e))?
        .into_inner();

    if !response.success {
        return Err(format!(
            "Lifecycle start_binary failed: service='{}', message='{}'",
            service_name, response.message
        ).into());
    }

    Ok(())
}

pub async fn stop_binary_by_service(service_name: &str, force: bool) -> Result<()> {
    let endpoint = lifecycle_endpoint();
    let mut client = BinaryLifecycleClient::connect(endpoint.clone())
        .await
        .map_err(|e| format!("Failed to connect to lifecycle service at {}: {}", endpoint, e))?;

    let timeout_secs = if force { 0 } else { 5 };
    let request = lifecycle::StopRequest {
        pid: 0,
        instance_id: String::new(),
        service_name: service_name.to_string(),
        stop_all: false,
        force,
        timeout_secs,
    };

    let response = client
        .stop_binary(Request::new(request))
        .await
        .map_err(|e| format!("gRPC stop_binary failed for service '{}': {}", service_name, e))?
        .into_inner();

    if !response.success {
        return Err(format!(
            "Lifecycle stop_binary failed: service='{}', message='{}'",
            service_name, response.message
        ).into());
    }

    Ok(())
}

/// Get status of a binary by service name
pub async fn get_status_by_service(service_name: &str) -> Result<lifecycle::StatusResponse> {
    let endpoint = lifecycle_endpoint();
    let mut client = BinaryLifecycleClient::connect(endpoint.clone())
        .await
        .map_err(|e| format!("Failed to connect to lifecycle service at {}: {}", endpoint, e))?;

    let request = lifecycle::StatusRequest {
        pid: 0,
        instance_id: String::new(),
        service_name: service_name.to_string(),
    };

    let response = client
        .get_status(Request::new(request))
        .await
        .map_err(|e| format!("gRPC get_status failed for service '{}': {}", service_name, e))?
        .into_inner();

    Ok(response)
}

/// Stop all binaries
pub async fn stop_all_binaries(force: bool) -> Result<()> {
    let endpoint = lifecycle_endpoint();
    let mut client = BinaryLifecycleClient::connect(endpoint.clone())
        .await
        .map_err(|e| format!("Failed to connect to lifecycle service at {}: {}", endpoint, e))?;

    let timeout_secs = if force { 0 } else { 5 };
    let request = lifecycle::StopRequest {
        pid: 0,
        instance_id: String::new(),
        service_name: String::new(),
        stop_all: true,
        force,
        timeout_secs,
    };

    let response = client
        .stop_binary(Request::new(request))
        .await
        .map_err(|e| format!("gRPC stop_all failed: {}", e))?
        .into_inner();

    if !response.success {
        return Err(format!(
            "Lifecycle stop_all failed: message='{}'",
            response.message
        ).into());
    }

    Ok(())
}
