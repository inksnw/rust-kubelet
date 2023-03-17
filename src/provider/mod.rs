use k8s_openapi::api::core::v1::Pod;
use tonic::transport::channel::Channel;
use tracing::*;

use cri::runtime_service_client::RuntimeServiceClient;

mod cri;


async fn get_client() -> RuntimeServiceClient<Channel> {
    let url = "http://192.168.50.231:8989";
    RuntimeServiceClient::connect(url).await.expect("Could not create client.")
}

pub async fn version() {
    let request = tonic::Request::new(cri::VersionRequest { version: "1".to_string() });
    let response = get_client().await
        .version(request)
        .await
        .expect("Request failed.");
    info!("使用的containerd版本为: {:?}", response.get_ref().version);
}

pub async fn run_pod(o: Pod) {
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
    let request = cri::CreateContainerRequest {
        pod_sandbox_id,
        config: Option::from(container_config),
        sandbox_config: Option::from(config.clone()),
    };
    let response = get_client().await
        .create_container(request)
        .await
        .expect("Request failed.");
    info!("容器创建成功,id: {}", response.get_ref().clone().container_id);

    fetch_status().await;
}

async fn fetch_status() {
    let request = cri::ListPodSandboxRequest { filter: None };
    let response = get_client().await
        .list_pod_sandbox(request)
        .await
        .expect("Request failed.");
    for i in &response.get_ref().items {
        info!("sandbox状态: {:?}",i );
    }
    let request = cri::ListContainersRequest { filter: None };
    let response = get_client().await
        .list_containers(request)
        .await
        .expect("Request failed.");
    for i in &response.get_ref().containers {
        info!("容器状态: {:?}",i );
    }
}