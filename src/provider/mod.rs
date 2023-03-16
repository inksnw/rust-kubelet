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
    println!("{:?}", response.get_ref().version);
}