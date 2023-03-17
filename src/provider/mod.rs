use tonic::transport::channel::Channel;
use tracing::*;

use cri::runtime_service_client::RuntimeServiceClient;

mod cri;
pub mod pod;


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
