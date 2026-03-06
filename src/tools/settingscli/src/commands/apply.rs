/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
//! Apply command — send a YAML file directly to the API Server

use crate::commands::{print_info, print_success};
use crate::{CliError, Result, SettingsClient};
use std::fs;
use std::path::Path;

/// Handle the `apply -f <file>` command
pub async fn handle(client: &SettingsClient, file: String) -> Result<()> {
    let content = read_file_content(&file)?;

    print_info(&format!("Applying {}...", file));

    client.post_yaml("/api/artifact", &content).await?;

    print_success(&format!("{} applied successfully", file));
    Ok(())
}

/// Read file content from a path or stdin ("-")
fn read_file_content(file_path: &str) -> Result<String> {
    if file_path == "-" {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        return Ok(buffer);
    }

    if !Path::new(file_path).exists() {
        return Err(CliError::Custom(format!("File not found: {}", file_path)));
    }

    let content = fs::read_to_string(file_path)?;
    Ok(content)
}
