use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, ListParams, ResourceExt, WatchEvent},
    Client, Config,
    runtime::{watcher, WatchStreamExt},
};
use tokio;
use tracing::*;

mod kubelet;
mod nodemod;
mod provider;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_file(true)
                .with_line_number(true),
        )
        .init();
    info!("Preparing kubelet config.");
    let local_config = Config::infer()
        .await
        .map_err(|e| anyhow::anyhow!("Unable to load config from host: {}", e))
        .expect("TODO: panic message");

    let kubelet_ins = kubelet::minikubelet::Kubelet::new(local_config).await;

    tokio::spawn(my_watch());
    kubelet_ins.start().await;
}

async fn my_watch() -> anyhow::Result<()> {
    let client = Client::try_default().await.unwrap();
    let pods: Api<Pod> = Api::namespaced(client, "default");
    let lp = ListParams::default();
    let mut stream = pods.watch(&lp, "0").await?.boxed();

    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(o) => {
                info!("Added {}", o.name_any());
            }
            WatchEvent::Modified(o) => {
                info!("update {}", o.name_any());
            }
            WatchEvent::Deleted(o) => {
                info!("delete {}", o.name_any());
            }
            _ => {}
        }
    }
    Ok(())
    //
    // watcher(pods, ListParams::default()).applied_objects()
    //     .try_for_each(|p| async move {
    //         println!("Applied: {}", p.name_any());
    //         Ok(())
    //     })
    //     .await.unwrap();
    // Ok(())
}