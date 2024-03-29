use std::time::Duration;

use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client};
use kube::api::PatchParams;
use tokio::time;
use tracing::*;

use crate::provider::{cri, get_client};
use crate::provider::cri::PodSandboxConfig;

pub async fn run_pod(o: Pod) {
    let (pod_sandbox_id, config) = create_sandbox(&o).await;
    let container_id = create_container(&o, &pod_sandbox_id, &config).await;
    start_container(&container_id).await;
    tokio::spawn(fetch_status_info());
}

pub async fn create_container(o: &Pod, pod_sandbox_id: &str, sandbox_config: &PodSandboxConfig) -> String {
    let name = o.clone().spec.unwrap().clone().containers[0].clone().name;
    let mut image = o.clone().spec.unwrap().clone().containers[0].clone().image.unwrap();
    image = format!("docker.io/library/{}:latest", image);

    let container_config = cri::ContainerConfig {
        metadata: Option::from(cri::ContainerMetadata { name, attempt: 0 }),
        image: Option::from(cri::ImageSpec { image, annotations: Default::default() }),
        command: vec![],
        args: vec![],
        working_dir: "".to_string(),
        envs: vec![],
        mounts: vec![],
        devices: vec![],
        labels: Default::default(),
        annotations: Default::default(),
        log_path: "/var/log/pods/container".to_string(),
        stdin: false,
        stdin_once: false,
        tty: false,
        linux: None,
        windows: None,
    };
    let s = pod_sandbox_id.clone().to_owned();
    let request = cri::CreateContainerRequest {
        pod_sandbox_id: s,
        config: Option::from(container_config),
        sandbox_config: Option::from(sandbox_config.clone()),
    };
    let response = get_client().await
        .create_container(request)
        .await
        .expect("Request failed.");
    info!("容器创建成功,id: {}", response.get_ref().clone().container_id);
    let container_id = response.get_ref().clone().container_id;
    container_id
}


pub async fn start_container(container_id: &str) {
    let request = cri::StartContainerRequest {
        container_id: container_id.parse().unwrap(),
    };
    get_client()
        .await
        .start_container(request)
        .await
        .expect("Request failed.");
    info!("启动容器成功,id: {}",container_id);
}

pub async fn create_sandbox(o: &Pod) -> (String, PodSandboxConfig) {
    let name = o.clone().metadata.name.unwrap();
    let config = cri::PodSandboxConfig {
        metadata: Option::from(cri::PodSandboxMetadata {
            name,
            uid: "123".to_string(),
            namespace: "default".to_string(),
            attempt: 0,
        }),
        hostname: "my_hostname".to_string(),
        log_directory: "/var/log/pods/sandbox".to_string(),
        dns_config: None,
        port_mappings: vec![],
        labels: Default::default(),
        annotations: Default::default(),
        linux: None,
        windows: None,
    };

    let request = cri::RunPodSandboxRequest { config: Option::from(config.clone()), runtime_handler: "".to_string() };
    let response = get_client().await
        .run_pod_sandbox(request)
        .await.map_err(|e| error!("创建sandbox失败: {}", e)).unwrap();
    let pod_sandbox_id = response.get_ref().clone().pod_sandbox_id;
    info!("沙箱容器id: {}", pod_sandbox_id);
    (pod_sandbox_id, config)
}

async fn fetch_status_info() {
    loop {
        let request = cri::ListPodSandboxRequest { filter: None };
        let response_sandbox = get_client().await
            .list_pod_sandbox(request)
            .await
            .expect("Request failed.");

        let request = cri::ListContainersRequest { filter: None };
        let response = get_client().await
            .list_containers(request)
            .await
            .expect("Request failed.");
        for i in &response.get_ref().containers {
            for j in &response_sandbox.get_ref().items {
                if i.pod_sandbox_id == j.id {
                    if i.state == 1 {
                        info!("{} 空间下的pod: {:?} 状态: {}",
                        j.metadata.clone().unwrap().namespace,j.metadata.clone().unwrap().name,"running");
                        update_status(j.metadata.clone().unwrap().name, j.metadata.clone().unwrap().namespace).await;
                    }
                }
            }
        }
        time::sleep(Duration::from_secs(4)).await;
    }
}

async fn update_status(name: String, ns: String) {
    let client = Client::try_default().await.unwrap();
    let status_patch = serde_json::json!({
    "status": {
        "phase": "Running",
        "qosClass": "BestEffort",
        "containerStatuses": [
            {
                "containerID": "containerd://bbe48a0007e9d2eca406acc6ba75041ca6e163bbd4d7490ab3248a5eeb1797be",
                "image": "docker.io/openebs/provisioner-localpv:3.3.0",
                "imageID": "docker.io/openebs/provisioner-localpv@sha256:9944beedeb5ad33b1013d62da026c3ac31f29238b378335e40ded2dcfe2c56f4",
                "name": "openebs-provisioner-hostpath",
                "ready": true,
                "restartCount": 1,
                "started": true,
                "state": {
                    "running": {
                        "startedAt": "2023-03-17T01:02:10Z"
                    }
                }
            }
        ]
    }
});
    let pod_client: Api<Pod> = Api::namespaced(client, &ns);
    pod_client.patch_status(
        &name,
        &PatchParams::default(),
        &kube::api::Patch::Strategic(status_patch),
    ).await.expect("TODO: panic message");
}