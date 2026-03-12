/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! ResourceManager gRPC client for sending resource requests from ApiServer.
//!
//! This module provides a client interface for the ApiServer to communicate with
//! the ResourceManager service via gRPC. It handles Network and Volume resource
//! creation requests parsed from YAML artifacts.

use common::resourcemanager::{
    connect_server, resource_manager_service_client::ResourceManagerServiceClient,
    NetworkResourceRequest, ResourceResponse, VolumeResourceRequest,
};
use tonic::{Request, Status};

/// ResourceManager gRPC client for ApiServer component.
///
/// This client manages the gRPC connection to the ResourceManager service and provides
/// methods for sending network and volume resource creation requests.
#[derive(Clone)]
pub struct ResourceManagerSender {
    /// Cached gRPC client connection to the ResourceManager service.
    client: Option<ResourceManagerServiceClient<tonic::transport::Channel>>,
}

impl Default for ResourceManagerSender {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceManagerSender {
    /// Creates a new ResourceManagerSender instance.
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Ensures a gRPC connection to the ResourceManager exists and is ready for use.
    async fn ensure_connected(&mut self) -> Result<(), Status> {
        if self.client.is_none() {
            match ResourceManagerServiceClient::connect(connect_server()).await {
                Ok(client) => {
                    self.client = Some(client);
                    Ok(())
                }
                Err(e) => Err(Status::unknown(format!(
                    "Failed to connect to ResourceManager: {}",
                    e
                ))),
            }
        } else {
            Ok(())
        }
    }

    /// Sends a network resource creation request to the ResourceManager.
    ///
    /// # Arguments
    /// * `request` - NetworkResourceRequest containing network configuration
    ///
    /// # Returns
    /// * `Result<tonic::Response<ResourceResponse>, Status>` - Response from ResourceManager
    pub async fn create_network(
        &mut self,
        request: NetworkResourceRequest,
    ) -> Result<tonic::Response<ResourceResponse>, Status> {
        self.ensure_connected().await?;

        if let Some(client) = &mut self.client {
            client.create_network_resource(Request::new(request)).await
        } else {
            Err(Status::unknown("ResourceManager client not connected"))
        }
    }

    /// Sends a volume resource creation request to the ResourceManager.
    ///
    /// # Arguments
    /// * `request` - VolumeResourceRequest containing volume configuration
    ///
    /// # Returns
    /// * `Result<tonic::Response<ResourceResponse>, Status>` - Response from ResourceManager
    pub async fn create_volume(
        &mut self,
        request: VolumeResourceRequest,
    ) -> Result<tonic::Response<ResourceResponse>, Status> {
        self.ensure_connected().await?;

        if let Some(client) = &mut self.client {
            client.create_volume_resource(Request::new(request)).await
        } else {
            Err(Status::unknown("ResourceManager client not connected"))
        }
    }
}
