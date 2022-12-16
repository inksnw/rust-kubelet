use std::{convert::TryFrom, path::Path, str};
use std::net;

use base64;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::certificates::v1::CertificateSigningRequest;
use kube::{Api, Config};
use kube::api::{ListParams, PostParams};
use kube_runtime::watcher::{Event, watcher};
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256, SanType};
use tokio::fs::write;
use tracing::info;

const APPROVED_TYPE: &str = "Approved";

pub async fn bootstrap() {
    let kubeconfig = Config::infer().await.map_err(|e| anyhow::anyhow!("Unable to load config from host: {}", e)).expect("TODO: panic message");
    bootstrap_tls(kubeconfig.clone()).await.expect("TODO: panic message");
}


async fn bootstrap_tls(kubeconfig: Config) -> anyhow::Result<()> {
    let cert_bundle = gen_tls_cert()?;
    let csr_name = format!("{}-tls", "my-imac");
    let client = kube::Client::try_from(kubeconfig)?;
    let csrs: Api<CertificateSigningRequest> = Api::all(client);
    let csr_json = serde_json::json!({
        "apiVersion": "certificates.k8s.io/v1",
        "kind": "CertificateSigningRequest",
        "metadata": {
            "name": csr_name,
        },
        "spec": {
        "request": base64::encode(cert_bundle.serialize_request_pem()?.as_bytes()),
        "signerName": "kubernetes.io/kubelet-serving",
        "usages": [
            "digital signature",
            "key encipherment",
            "server auth"
        ]
        }
    });
    let post_data =
        serde_json::from_value(csr_json).expect("Invalid CSR JSON, this is a programming error");
    csrs.create(&PostParams::default(), &post_data).await?;
    let inf = watcher(
        csrs,
        ListParams::default().fields(&format!("metadata.name={}", csr_name)),
    );
    let mut watcher = inf.boxed();
    let mut certificate = String::new();
    let mut got_cert = false;
    let start = std::time::Instant::now();
    while let Some(event) = watcher.try_next().await? {
        let status = match event {
            Event::Applied(m) => m.status.unwrap(),
            Event::Restarted(mut certs) => {
                if certs.len() > 1 {
                    return Err(anyhow::anyhow!("On watch restart, got more than 1 authentication CSR. This means something is in an incorrect state"));
                }
                certs.remove(0).status.unwrap()
            }
            Event::Deleted(_) => {
                return Err(anyhow::anyhow!( "Authentication CSR was deleted before it was approved"));
            }
        };

        if let Some(cert) = status.certificate {
            if let Some(v) = status.conditions {
                if v.into_iter().any(|c| c.type_.as_str() == APPROVED_TYPE) {
                    certificate = std::str::from_utf8(&cert.0)?.to_owned();
                    got_cert = true;
                    break;
                }
            }
        }
        info!(remaining = ?start.elapsed(), "Got modified event, but CSR for serving certs is not currently approved");
    }
    if !got_cert {
        return Err(anyhow::anyhow!(
            "Authentication certificates were never approved"
        ));
    }
    let private_key = cert_bundle.serialize_private_key_pem();
    let cert_file = Path::new("./mycert.crt");
    let key_file = Path::new("./mycert.key");
    write(&cert_file, &certificate).await?;
    write(&key_file, &private_key).await?;
    Ok(())
}


fn gen_tls_cert() -> anyhow::Result<Certificate> {
    let mut params = CertificateParams::default();
    params.not_before = chrono::Utc::now();
    params.not_after = chrono::Utc::now() + chrono::Duration::weeks(52);
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::OrganizationName, "system:nodes");
    distinguished_name.push(DnType::CommonName, &format!("system:node:{}", "my-imac"),
    );
    params.distinguished_name = distinguished_name;
    params.key_pair.replace(KeyPair::generate(&PKCS_ECDSA_P256_SHA256)?);
    params.alg = &PKCS_ECDSA_P256_SHA256;
    let ip = net::IpAddr::V4(net::Ipv4Addr::new(192, 168, 50, 251));
    params.subject_alt_names = vec![
        SanType::DnsName("my-imac".to_string()),
        SanType::IpAddress(ip),
    ];
    Ok(Certificate::from_params(params)?)
}