/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
//! Delete command — remove a named resource via the API Server

use crate::commands::{print_info, print_success};
use crate::{CliError, Result, SettingsClient};

/// Handle the `delete <resource_type> <name>` command
pub async fn handle(client: &SettingsClient, resource_type: String, name: String) -> Result<()> {
    let endpoint = match resource_type.to_lowercase().as_str() {
        "scenario" => format!("/api/v1/scenarios/{}", name),
        "package" => format!("/api/v1/packages/{}", name),
        _ => return Err(CliError::InvalidResourceType(resource_type)),
    };

    print_info(&format!("Deleting {} '{}'...", resource_type, name));

    client.delete(&endpoint).await?;

    print_success(&format!("{} '{}' deleted", resource_type, name));
    Ok(())
}
