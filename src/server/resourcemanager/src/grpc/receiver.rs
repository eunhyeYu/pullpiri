/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::grpc::sender::csi::CsiSender;
use crate::grpc::sender::pharos::PharosSender;
use common::external::csi::{VolumeCreateRequest, VolumeDeleteRequest};
use common::external::pharos::{NetworkRemoveRequest, NetworkSetupRequest};
use common::resourcemanager::resource_manager_service_server::ResourceManagerService;
use common::resourcemanager::{
    DeleteResourceRequest, NetworkResourceRequest, ResourceResponse, VolumeResourceRequest,
};
use tonic::Response;

#[allow(dead_code)]
pub struct ResourceManagerGrpcServer {
    /// Pharos sender for network resource operations
    pharos_sender: PharosSender,
    /// CSI sender for volume resource operations
    csi_sender: CsiSender,
}

#[allow(dead_code)]
impl ResourceManagerGrpcServer {
    /// Creates a new ResourceManagerGrpcServer instance
    pub fn new() -> Self {
        Self {
            pharos_sender: PharosSender::new(),
            csi_sender: CsiSender::new(),
        }
    }
}

#[tonic::async_trait]
impl ResourceManagerService for ResourceManagerGrpcServer {
    async fn create_network_resource(
        &self,
        request: tonic::Request<NetworkResourceRequest>,
    ) -> Result<tonic::Response<ResourceResponse>, tonic::Status> {
        let req = request.into_inner();

        println!("RESOURCE MANAGER: Processing Network Resource");
        println!("Network Name: {}", &req.network_name);
        println!("Network Mode: {}", &req.network_mode);

        // Convert to Pharos request
        let pharos_req = NetworkSetupRequest {
            network_name: req.network_name,
            network_mode: req.network_mode,
        };

        println!("Sending NetworkSetupRequest to Pharos:");
        println!("Network Name: {}", &pharos_req.network_name);
        println!("Network Mode: {}", &pharos_req.network_mode);

        let network_name = pharos_req.network_name.clone();
        match self.pharos_sender.clone().setup_network(pharos_req).await {
            Ok(response) => {
                let resp = response.into_inner();
                println!(
                    "Successfully created network resource: {}",
                    &network_name
                );
                Ok(Response::new(ResourceResponse {
                    success: resp.success,
                    message: resp.message,
                }))
            }
            Err(e) => {
                println!("Failed to create network resource: {:?}", e);
                Ok(Response::new(ResourceResponse {
                    success: false,
                    message: format!("Failed to create network resource: {}", e),
                }))
            }
        }
    }

    async fn create_volume_resource(
        &self,
        request: tonic::Request<VolumeResourceRequest>,
    ) -> Result<tonic::Response<ResourceResponse>, tonic::Status> {
        let req = request.into_inner();

        println!("RESOURCE MANAGER: Processing Volume Resource");
        println!("Volume Name: {}", &req.volume_name);
        println!("Capacity: {}", &req.capacity);
        println!("Mount Path: {}", &req.mountpath);
        println!("ASIL Level: {}", &req.asil_level);

        // Convert to CSI request
        let csi_req = VolumeCreateRequest {
            volume_name: req.volume_name,
            capacity: req.capacity,
            mountpath: req.mountpath,
            asil_level: req.asil_level,
        };

        println!("Sending VolumeCreateRequest to CSI:");
        println!("Volume Name: {}", &csi_req.volume_name);
        println!("Capacity: {}", &csi_req.capacity);
        println!("Mount Path: {}", &csi_req.mountpath);
        println!("ASIL Level: {}", &csi_req.asil_level);

        let volume_name = csi_req.volume_name.clone();
        match self.csi_sender.clone().create_volume(csi_req).await {
            Ok(response) => {
                let resp = response.into_inner();
                println!(
                    "Successfully created volume resource: {}",
                    &volume_name
                );
                Ok(Response::new(ResourceResponse {
                    success: resp.success,
                    message: resp.message,
                }))
            }
            Err(e) => {
                println!("Failed to create volume resource: {:?}", e);
                Ok(Response::new(ResourceResponse {
                    success: false,
                    message: format!("Failed to create volume resource: {}", e),
                }))
            }
        }
    }

    async fn delete_network_resource(
        &self,
        request: tonic::Request<DeleteResourceRequest>,
    ) -> Result<tonic::Response<ResourceResponse>, tonic::Status> {
        let req = request.into_inner();

        println!("RESOURCE MANAGER: Deleting Network Resource");
        println!("Resource Name: {}", &req.resource_name);

        // Convert to Pharos request
        let pharos_req = NetworkRemoveRequest {
            network_name: req.resource_name,
        };

        println!("Sending NetworkRemoveRequest to Pharos:");
        println!("Network Name: {}", &pharos_req.network_name);

        let resource_name = pharos_req.network_name.clone();
        match self.pharos_sender.clone().remove_network(pharos_req).await {
            Ok(response) => {
                let resp = response.into_inner();
                println!(
                    "Successfully deleted network resource: {}",
                    &resource_name
                );
                Ok(Response::new(ResourceResponse {
                    success: resp.success,
                    message: resp.message,
                }))
            }
            Err(e) => {
                println!("Failed to delete network resource: {:?}", e);
                Ok(Response::new(ResourceResponse {
                    success: false,
                    message: format!("Failed to delete network resource: {}", e),
                }))
            }
        }
    }

    async fn delete_volume_resource(
        &self,
        request: tonic::Request<DeleteResourceRequest>,
    ) -> Result<tonic::Response<ResourceResponse>, tonic::Status> {
        let req = request.into_inner();

        println!("RESOURCE MANAGER: Deleting Volume Resource");
        println!("Resource Name: {}", &req.resource_name);

        // Convert to CSI request
        let csi_req = VolumeDeleteRequest {
            volume_name: req.resource_name,
        };

        println!("Sending VolumeDeleteRequest to CSI:");
        println!("Volume Name: {}", &csi_req.volume_name);

        let resource_name = csi_req.volume_name.clone();
        match self.csi_sender.clone().delete_volume(csi_req).await {
            Ok(response) => {
                let resp = response.into_inner();
                println!(
                    "Successfully deleted volume resource: {}",
                    &resource_name
                );
                Ok(Response::new(ResourceResponse {
                    success: resp.success,
                    message: resp.message,
                }))
            }
            Err(e) => {
                println!("Failed to delete volume resource: {:?}", e);
                Ok(Response::new(ResourceResponse {
                    success: false,
                    message: format!("Failed to delete volume resource: {}", e),
                }))
            }
        }
    }
}
