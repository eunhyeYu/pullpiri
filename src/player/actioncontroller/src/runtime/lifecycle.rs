/*
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::Result;
use common::logd;
use common::spec::artifact::Binary;
use serde_yaml::Value;

/// Restart policy options parsed from YAML
#[derive(Debug, Clone, Copy, Default)]
pub struct RestartConfig {
    pub policy: i32,       // 0=Never, 1=OnFailure, 2=Always
    pub max_retries: u32,
    pub restart_delay_secs: u32,
}

// ============================================================================
// Binary artifact direct handlers (kind: Binary)
// ============================================================================

/// Start a Binary artifact workload
pub async fn start_binary_artifact(binary: &Binary) -> Result<()> {
    let spec = binary.get_spec();
    let service_name = binary.get_name();
    
    logd!(3, "Starting lifecycle binary: service={}, path={}", service_name, spec.path);
    
    let restart_config = RestartConfig {
        policy: spec.restart_policy.to_proto_value(),
        max_retries: spec.max_retries,
        restart_delay_secs: spec.restart_delay_secs,
    };

    crate::grpc::sender::lifecycle::start_binary(
        &service_name,
        &spec.path,
        spec.args.clone(),
        restart_config,
    ).await?;
    
    logd!(2, "Lifecycle Binary start succeeded: service={}, pid info available via get_status", service_name);
    Ok(())
}

/// Stop a Binary artifact workload
pub async fn stop_binary_artifact(binary: &Binary) -> Result<()> {
    let service_name = binary.get_name();
    logd!(3, "Stopping lifecycle binary: service={}", service_name);
    
    crate::grpc::sender::lifecycle::stop_binary_by_service(&service_name, false).await?;
    logd!(2, "Lifecycle Binary stop succeeded: service={}", service_name);
    Ok(())
}

/// Restart a Binary artifact workload
pub async fn restart_binary_artifact(binary: &Binary) -> Result<()> {
    let service_name = binary.get_name();
    logd!(3, "Restarting lifecycle binary: service={}", service_name);

    // Try graceful stop first, ignore errors as process might not be running
    if let Err(e) = crate::grpc::sender::lifecycle::stop_binary_by_service(&service_name, false).await {
        logd!(4, "Lifecycle Binary stop before restart failed (ignored): service={}, err={}", service_name, e);
    }

    let spec = binary.get_spec();
    let restart_config = RestartConfig {
        policy: spec.restart_policy.to_proto_value(),
        max_retries: spec.max_retries,
        restart_delay_secs: spec.restart_delay_secs,
    };

    crate::grpc::sender::lifecycle::start_binary(
        &service_name,
        &spec.path,
        spec.args.clone(),
        restart_config,
    ).await?;
    logd!(2, "Lifecycle Binary restart succeeded: service={}", service_name);
    Ok(())
}

/// Get status of a Binary artifact workload
pub async fn get_binary_status(binary: &Binary) -> Result<crate::grpc::sender::lifecycle::lifecycle::StatusResponse> {
    let service_name = binary.get_name();
    crate::grpc::sender::lifecycle::get_status_by_service(&service_name).await
}

// ============================================================================
// Pod/Model YAML handlers (legacy compatibility)
// ============================================================================

pub async fn start_workload(pod_yaml: &str) -> Result<()> {
    let spec = parse_start_spec(pod_yaml)?;
    crate::grpc::sender::lifecycle::start_binary(
        &spec.service_name,
        &spec.binary_path,
        spec.args,
        spec.restart_config,
    ).await?;
    logd!(2, "Lifecycle start succeeded: service={}", spec.service_name);
    Ok(())
}

pub async fn stop_workload(pod_yaml: &str) -> Result<()> {
    let service_name = parse_service_name(pod_yaml)?;
    crate::grpc::sender::lifecycle::stop_binary_by_service(&service_name, false).await?;
    logd!(2, "Lifecycle stop succeeded: service={}", service_name);
    Ok(())
}

pub async fn restart_workload(pod_yaml: &str) -> Result<()> {
    let spec = parse_start_spec(pod_yaml)?;

    if let Err(e) = crate::grpc::sender::lifecycle::stop_binary_by_service(&spec.service_name, false).await {
        logd!(4, "Lifecycle stop before restart failed (ignored): service={}, err={}", spec.service_name, e);
    }

    crate::grpc::sender::lifecycle::start_binary(
        &spec.service_name,
        &spec.binary_path,
        spec.args,
        spec.restart_config,
    ).await?;
    logd!(2, "Lifecycle restart succeeded: service={}", spec.service_name);
    Ok(())
}

/// Parsed binary start specification
struct StartSpec {
    service_name: String,
    binary_path: String,
    args: Vec<String>,
    restart_config: RestartConfig,
}

fn parse_start_spec(pod_yaml: &str) -> Result<StartSpec> {
    let root: Value = serde_yaml::from_str(pod_yaml)?;
    let service_name = extract_service_name(&root)?;
    let spec_node = root.get("spec");

    let first_container = spec_node
        .and_then(|v| v.get("containers"))
        .and_then(Value::as_sequence)
        .and_then(|containers| containers.first())
        .ok_or_else(|| "pod spec must contain spec.containers[0]".to_string())?;

    let command = parse_string_list(first_container.get("command"));
    if command.is_empty() {
        return Err("lifecycle runtime requires spec.containers[0].command".into());
    }

    let mut all_args = if command.len() > 1 {
        command[1..].to_vec()
    } else {
        Vec::new()
    };

    let mut pod_args = parse_string_list(first_container.get("args"));
    all_args.append(&mut pod_args);

    // Parse restart config from spec level
    let restart_config = parse_restart_config(spec_node);

    Ok(StartSpec {
        service_name,
        binary_path: command[0].clone(),
        args: all_args,
        restart_config,
    })
}

fn parse_service_name(pod_yaml: &str) -> Result<String> {
    let root: Value = serde_yaml::from_str(pod_yaml)?;
    extract_service_name(&root)
}

fn extract_service_name(root: &Value) -> Result<String> {
    root.get("metadata")
        .and_then(|v| v.get("name"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| "pod metadata.name is required".into())
}

fn parse_string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Sequence(seq)) => seq
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        Some(Value::String(s)) => s
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

/// Parse restart configuration from YAML spec
/// Looks for:
///   spec.restartPolicy: Never | OnFailure | Always
///   spec.maxRetries: u32 (default 0)
///   spec.restartDelaySecs: u32 (default 0)
fn parse_restart_config(spec_node: Option<&Value>) -> RestartConfig {
    let Some(spec) = spec_node else {
        return RestartConfig::default();
    };

    let policy = spec
        .get("restartPolicy")
        .and_then(Value::as_str)
        .map(|s| match s.to_ascii_lowercase().as_str() {
            "always" => 2,
            "onfailure" | "on_failure" => 1,
            _ => 0, // Never
        })
        .unwrap_or(0);

    let max_retries = spec
        .get("maxRetries")
        .and_then(Value::as_u64)
        .map(|v| v as u32)
        .unwrap_or(0);

    let restart_delay_secs = spec
        .get("restartDelaySecs")
        .and_then(Value::as_u64)
        .map(|v| v as u32)
        .unwrap_or(0);

    RestartConfig {
        policy,
        max_retries,
        restart_delay_secs,
    }
}
