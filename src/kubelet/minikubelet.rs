use chrono::Utc;
use k8s_openapi::api::coordination::v1::Lease;
use k8s_openapi::api::core::v1::Node as KubeNode;
use kube::api::{Api, PatchParams, PostParams};
use kube::Error;
use kube::error::ErrorResponse;
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
        match node_client.get("my-imac").await {
            Ok(_) => {
                info!("节点已经存在,更新租约");
                let uid = self.uid(&client.clone(), "my-imac").await;
                self.update(&uid, "my-imac").await.expect("TODO: panic message");
                return;
            }
            Err(Error::Api(ErrorResponse { code: 404, .. })) => {
                self.create().await;
            }
            Err(e) => {
                error!(
                error = %e,
                "Exhausted retries when trying to talk to API. Not retrying"
            );
            }
        }
    }

    async fn create(&self) {
        let client = kube::Client::try_from(self.kube_config.clone()).unwrap();
        let node_client: Api<KubeNode> = Api::all(client.clone());
        let mut builder = nodemod::node::Node::builder();
        builder.set_name("my-imac");
        builder.add_annotation("node.alpha.kubernetes.io/ttl", "0");
        builder.add_annotation("volumes.kubernetes.io/controller-managed-attach-detach", "true");
        builder.add_label("kubernetes.io/hostname", "my-imac");
        builder.add_capacity("cpu", "4");

        let node = builder.build().into_inner();

        match node_client.create(&PostParams::default(), &node).await {
            Ok(node) => {
                let node_uid = node.metadata.uid.unwrap();
                create_lease(&node_uid, "my-imac", &client).await;
                info!("Successfully created node");
            }
            Err(e) => {
                error!(
                error = %e,
                "Exhausted retries creating node after failed create. Not retrying"
            );
                return;
            }
        }
    }

    pub async fn uid(&self, client: &kube::Client, node_name: &str) -> String {
        let node_client: Api<KubeNode> = Api::all(client.clone());
        let node = node_client.get(node_name).await.unwrap();
        node.metadata.uid.unwrap()
    }


    async fn update(&self, node_uid: &str, node_name: &str) -> Result<Lease, Error> {
        let client = kube::Client::try_from(self.kube_config.clone()).unwrap();
        let leases: Api<Lease> = Api::namespaced(client.clone(), "kube-node-lease");
        let lease = lease_definition(node_uid, node_name);
        let resp = leases.patch(
            node_name,
            &PatchParams::default(),
            &kube::api::Patch::Strategic(lease)).await;
        match &resp {
            Ok(_) => info!("租约更新成功"),
            Err(e) => error!("更新租约失败 {e}"),
        }
        resp
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
