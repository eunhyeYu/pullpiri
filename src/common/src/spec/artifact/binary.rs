/*
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

//! Binary artifact specification for lifecycle-managed executables

use super::Artifact;
use super::Binary;
use serde::{Deserialize, Serialize};

/// Specification for a binary executable workload
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BinarySpec {
    /// Path to the executable binary
    pub path: String,
    /// Command-line arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Restart policy: Never, OnFailure, Always
    #[serde(default, rename = "restartPolicy")]
    pub restart_policy: RestartPolicy,
    /// Maximum retry count (0 = unlimited when policy allows)
    #[serde(default, rename = "maxRetries")]
    pub max_retries: u32,
    /// Delay in seconds before restart
    #[serde(default, rename = "restartDelaySecs")]
    pub restart_delay_secs: u32,
    /// Target node name
    #[serde(default)]
    pub node: Option<String>,
}

/// Restart policy enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum RestartPolicy {
    #[default]
    Never,
    OnFailure,
    Always,
}

impl RestartPolicy {
    /// Convert to gRPC protocol value
    pub fn to_proto_value(&self) -> i32 {
        match self {
            RestartPolicy::Never => 0,
            RestartPolicy::OnFailure => 1,
            RestartPolicy::Always => 2,
        }
    }
}

impl Artifact for Binary {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Binary {
    pub fn get_name(&self) -> String {
        self.metadata.name.clone()
    }

    pub fn get_spec(&self) -> &BinarySpec {
        &self.spec
    }

    pub fn get_node(&self) -> Option<&str> {
        self.spec.node.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_binary(name: &str) -> Binary {
        let yaml = format!(
            r#"
apiVersion: v1
kind: Binary
metadata:
  name: {}
spec:
  path: /bin/sleep
  args: ["30"]
  restartPolicy: OnFailure
  maxRetries: 3
  restartDelaySecs: 1
  node: test-node
"#,
            name
        );
        serde_yaml::from_str(&yaml).unwrap()
    }

    #[test]
    fn test_binary_get_name() {
        let binary = create_test_binary("test-binary");
        assert_eq!(binary.get_name(), "test-binary");
    }

    #[test]
    fn test_binary_spec_fields() {
        let binary = create_test_binary("spec-test");
        let spec = binary.get_spec();
        assert_eq!(spec.path, "/bin/sleep");
        assert_eq!(spec.args, vec!["30"]);
        assert_eq!(spec.restart_policy, RestartPolicy::OnFailure);
        assert_eq!(spec.max_retries, 3);
        assert_eq!(spec.restart_delay_secs, 1);
    }

    #[test]
    fn test_restart_policy_to_proto() {
        assert_eq!(RestartPolicy::Never.to_proto_value(), 0);
        assert_eq!(RestartPolicy::OnFailure.to_proto_value(), 1);
        assert_eq!(RestartPolicy::Always.to_proto_value(), 2);
    }

    #[test]
    fn test_binary_get_node() {
        let binary = create_test_binary("node-test");
        assert_eq!(binary.get_node(), Some("test-node"));
    }
}
