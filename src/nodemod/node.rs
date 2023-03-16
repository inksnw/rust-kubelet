#![allow(dead_code)]

use std::collections::BTreeMap;

use k8s_openapi::api::core::v1::Node as KubeNode;

pub struct Node(k8s_openapi::api::core::v1::Node);

impl Node {
    pub fn builder() -> Builder {
        Default::default()
    }
    pub fn into_inner(self) -> KubeNode { self.0 }
}

impl Builder {
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
    pub fn add_annotation(&mut self, key: &str, value: &str) {
        self.annotations.insert(key.to_string(), value.to_string());
    }
    pub fn add_label(&mut self, key: &str, value: &str) {
        self.labels.insert(key.to_string(), value.to_string());
    }
    pub fn add_capacity(&mut self, key: &str, value: &str) {
        self.capacity.insert(
            key.to_string(),
            k8s_openapi::apimachinery::pkg::api::resource::Quantity(value.to_string()),
        );
    }

    pub fn build(self) -> Node {
        let metadata = k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(self.name),
            annotations: Some(self.annotations),
            labels: Some(self.labels),
            ..Default::default()
        };
        let spec = k8s_openapi::api::core::v1::NodeSpec {
            pod_cidr: Some(self.pod_cidr),
            taints: Some(self.taints),
            ..Default::default()
        };

        let node_info = k8s_openapi::api::core::v1::NodeSystemInfo {
            architecture: self.architecture,
            kube_proxy_version: self.kube_proxy_version,
            kubelet_version: self.kubelet_version,
            container_runtime_version: self.container_runtime_version,
            operating_system: self.operating_system,
            ..Default::default()
        };

        let status = k8s_openapi::api::core::v1::NodeStatus {
            node_info: Some(node_info),
            capacity: Some(self.capacity),
            allocatable: Some(self.allocatable),
            daemon_endpoints: Some(k8s_openapi::api::core::v1::NodeDaemonEndpoints {
                kubelet_endpoint: Some(k8s_openapi::api::core::v1::DaemonEndpoint {
                    port: self.port,
                }),
            }),
            conditions: Some(self.conditions),
            addresses: Some(self.addresses),
            ..Default::default()
        };
        let kube_node = k8s_openapi::api::core::v1::Node {
            metadata,
            spec: Some(spec),
            status: Some(status),
        };
        Node(kube_node)
    }
}


/// Builder for node definition.
pub struct Builder {
    name: String,
    annotations: BTreeMap<String, String>,
    labels: BTreeMap<String, String>,
    pod_cidr: String,
    taints: Vec<k8s_openapi::api::core::v1::Taint>,
    architecture: String,
    kube_proxy_version: String,
    kubelet_version: String,
    container_runtime_version: String,
    operating_system: String,
    capacity: BTreeMap<String, k8s_openapi::apimachinery::pkg::api::resource::Quantity>,
    allocatable: BTreeMap<String, k8s_openapi::apimachinery::pkg::api::resource::Quantity>,
    port: i32,
    conditions: Vec<k8s_openapi::api::core::v1::NodeCondition>,
    addresses: Vec<k8s_openapi::api::core::v1::NodeAddress>,
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            name: "krustlet".to_string(),
            annotations: BTreeMap::new(),
            labels: BTreeMap::new(),
            pod_cidr: "10.244.0.0/24".to_string(),
            taints: vec![],
            architecture: "".to_string(),
            kube_proxy_version: "v1.17.0".to_string(),
            kubelet_version: "v1.24.7".to_string(),
            container_runtime_version: "mvp".to_string(),
            operating_system: "linux".to_string(),
            capacity: BTreeMap::new(),
            allocatable: BTreeMap::new(),
            port: 10250,
            conditions: vec![],
            addresses: vec![],
        }
    }
}