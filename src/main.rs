use kube::Config;
use tokio;
use tracing::*;

mod kubelet;
mod nodemod;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt().event_format(
        tracing_subscriber::fmt::format()
            .with_file(true)
            .with_line_number(true)
    ).init();
    info!("Preparing kubelet config.");
    let local_config = Config::infer().await
        .map_err(|e| anyhow::anyhow!("Unable to load config from host: {}", e))
        .expect("TODO: panic message");
    //生成证书
    // kubelet::bootstrapping::bootstrap_tls(local_config.clone()).await.expect("TODO: panic message");

    let kubelet_ins = kubelet::minikubelet::Kubelet::new(local_config).await;
    kubelet_ins.start().await;
}
