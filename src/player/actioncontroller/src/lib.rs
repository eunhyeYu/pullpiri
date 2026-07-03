/*
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

//! Action Controller Library
//! 
//! This library provides the core functionality for the Action Controller component.

pub mod grpc;
pub mod runtime;
pub mod manager;

// Re-export commonly used types for convenience
pub use runtime::lifecycle::{RestartConfig, start_binary_artifact, stop_binary_artifact, get_binary_status};
