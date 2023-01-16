use chrono::Utc;
use k8s_openapi::api::coordination::v1::Lease;
use k8s_openapi::api::core::v1::Node as KubeNode;
use kube::Api;
use kube::api::PostParams;
use tracing::{error, info};

use crate::nodemod;

pub struct Kubelet {
    kube_config: kube::Config,
}

impl Kubelet {
    pub async fn new(kube_config: kube::Config) -> Self {
        Kubelet { kube_config }
    }
    pub async fn start(&self) {
        let client = kube::Client::try_from(self.kube_config.clone()).unwrap();
        let node_client: Api<KubeNode> = Api::all(client.clone());
        let mut builder = nodemod::node::Node::builder();
        builder.set_name("my-imac");
        builder.add_annotation("node.alpha.kubernetes.io/ttl", "0");
        builder.add_annotation("volumes.kubernetes.io/controller-managed-attach-detach", "true");
        builder.add_label("kubernetes.io/hostname", "my-imac");
        builder.add_capacity("cpu", "4");

        let node = builder.build().into_inner();

        node_client.create(&PostParams::default(), &node).await.expect("TODO: panic message");
        let node_uid = "fced67d4-e649-41c3-943a-cc6a43fcdf41".to_string();
        create_lease(&node_uid, "my-imac", &client).await;
        info!("Successfully created node");
    }
}

async fn create_lease(node_uid: &str, node_name: &str, client: &kube::Client) {
    let leases: Api<Lease> = Api::namespaced(client.clone(), "kube-node-lease");
    let lease = lease_definition(node_uid, node_name);
    let lease = serde_json::from_value(lease)
        .expect("failed to deserialize lease from lease definition JSON");
    match leases.create(&PostParams::default(), &lease).await {
        Ok(_) => {
            info!("Created lease for node");
        }
        _ => {
            error!("Created lease for node failed");
        }
    }
}

fn lease_definition(node_uid: &str, node_name: &str) -> serde_json::Value {
    serde_json::json!(
        {
            "apiVersion": "coordination.k8s.io/v1",
            "kind": "Lease",
            "metadata": {
                "name": node_name,
                "ownerReferences": [
                    {
                        "apiVersion": "v1",
                        "kind": "Node",
                        "name": node_name,
                        "uid": node_uid
                    }
                ]
            },
            "spec": lease_spec_definition(node_name)
        }
    )
}

fn lease_spec_definition(node_name: &str) -> serde_json::Value {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    serde_json::json!(
        {
            "holderIdentity": node_name,
            "acquireTime": now,
            "renewTime": now,
            "leaseDurationSeconds": 300
        }
    )
}
