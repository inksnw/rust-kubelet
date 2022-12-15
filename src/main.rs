use tokio;
use tracing::*;

mod kubelet;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt().event_format(
        tracing_subscriber::fmt::format()
            .with_file(true)
            .with_line_number(true)
    ).init();
    info!("Preparing kubelet config.");
    kubelet::bootstrapping::bootstrap().await;
}
