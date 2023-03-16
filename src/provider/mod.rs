use k8s_openapi::api::core::v1::Pod;
use tracing::*;

mod cri;

pub async fn version() {
    let url = "http://192.168.50.231:8989";
    let mut client = cri::runtime_service_client::RuntimeServiceClient::connect(url)
        .await
        .expect("Could not create client.");
    let request = tonic::Request::new(cri::VersionRequest { version: "1".to_string() });
    let response = client
        .version(request)
        .await
        .expect("Request failed.");
    info!("使用的containerd版本为: {:?}", response.get_ref().version);
}

pub async fn run_pod(o: Pod) {
    let url = "http://192.168.50.231:8989";
    let mut client = cri::runtime_service_client::RuntimeServiceClient::connect(url)
        .await
        .expect("Could not create client.");
    let config = cri::PodSandboxConfig {
        metadata: Option::from(cri::PodSandboxMetadata {
            name: "my_sandbox".to_string(),
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
    let response = client
        .run_pod_sandbox(request)
        .await
        .expect("Request failed.");
    let pod_sandbox_id = response.get_ref().clone().pod_sandbox_id;
    info!("沙箱容器id: {:?}", pod_sandbox_id);

    let name = o.metadata.name.unwrap();
    let mut image = o.spec.unwrap().containers[0].clone().image.unwrap();
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
    let response = client
        .create_container(request)
        .await
        .expect("Request failed.");
    info!("容器创建成功,id: {:?}", response.get_ref().clone().container_id);
}