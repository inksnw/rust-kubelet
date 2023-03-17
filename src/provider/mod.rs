use tonic::transport::channel::Channel;

use cri::runtime_service_client::RuntimeServiceClient;

mod cri;
pub mod pod;


async fn get_client() -> RuntimeServiceClient<Channel> {
    let url = "http://192.168.50.231:8989";
    RuntimeServiceClient::connect(url).await.expect("Could not create client.")
}

