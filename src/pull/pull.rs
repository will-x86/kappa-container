use anyhow::{anyhow, Result};
use log::{debug, info};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde_json::Value;
use std::fs::{self, File};
use std::io::copy;
use std::path::Path;
#[allow(dead_code)]
fn map_architecture(arch: &str) -> &'static str {
    match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64v8",
        "armv7l" => "arm32v7",
        "armv6l" => "arm32v6",
        "armv5l" => "arm32v5",
        "ppc64le" => "ppc64le",
        "s390x" => "s390x",
        "mips64" => "mips64le",
        "riscv64" => "riscv64",
        "i686" => "i386",
        _ => "unknown",
    }
}

pub async fn pull(image: &str) -> Result<()> {
    let (namespace, repository, version) = if image.contains('/') {
        let parts: Vec<&str> = image.split('/').collect();
        if parts.len() >= 3 {
            (
                parts[0].to_string(),
                parts[1].to_string(),
                parts[2..].join("/"),
            )
        } else {
            (
                parts[0].to_string(),
                parts[1..].join("/"),
                "latest".to_string(),
            )
        }
    } else {
        (
            "library".to_string(),
            image.to_string(),
            "latest".to_string(),
        )
    };

    info!(
        "Attempting to pull image with namespace '{}' and repository '{}' and version '{}'",
        namespace, repository, version
    );

    let client = reqwest::Client::new();
    let registry = "https://registry-1.docker.io";
    let auth_url = "https://auth.docker.io/token";

    // Get authentication token
    let token_url = format!(
        "{}?service=registry.docker.io&scope=repository:{}/{}:pull",
        auth_url, namespace, repository
    );
    let token_response: Value = client.get(&token_url).send().await?.json().await?;
    let token = token_response["token"]
        .as_str()
        .ok_or_else(|| anyhow!("Failed to get token"))?;

    // Get manifest list
    let manifest_url = format!(
        "{}/v2/{}/{}/manifests/{}",
        registry, namespace, repository, version
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static(
            "application/vnd.docker.distribution.manifest.list.v2+json,\
             application/vnd.docker.distribution.manifest.v2+json,\
             application/vnd.oci.image.manifest.v1+json,\
             application/vnd.oci.image.index.v1+json",
        ),
    );

    let manifest_list: Value = client
        .get(&manifest_url)
        .headers(headers.clone())
        .send()
        .await?
        .json()
        .await?;

    debug!(
        "Got manifest list {:?}",
        serde_json::to_string(&manifest_list.clone())?
    );

    // Check if it's a manifest list or a single manifest
    let manifest = if manifest_list["manifests"].is_array() {
        // It's a manifest list, select the appropriate manifest for the current architecture
        let current_arch = std::env::consts::ARCH;
        let mapped_arch = map_architecture(current_arch);

        let manifest_digest = manifest_list["manifests"]
            .as_array()
            .ok_or_else(|| anyhow!("No manifests found"))?
            .iter()
            .find(|m| m["platform"]["architecture"].as_str() == Some(mapped_arch))
            .ok_or_else(|| anyhow!("No manifest found for current architecture"))?["digest"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid manifest digest"))?;

        // Fetch the architecture-specific manifest
        let arch_manifest_url = format!(
            "{}/v2/{}/{}/manifests/{}",
            registry, namespace, repository, manifest_digest
        );
        client
            .get(&arch_manifest_url)
            .headers(headers.clone())
            .send()
            .await?
            .json()
            .await?
    } else {
        // It's already a single manifest
        manifest_list
    };

    debug!("{:?}", manifest);
    let config_digest = manifest["config"]["digest"]
        .as_str()
        .ok_or_else(|| anyhow!("Failed to get config digest"))?;

    // Download config
    let config_url = format!(
        "{}/v2/{}/{}/blobs/{}",
        registry, namespace, repository, config_digest
    );
    let config_response = client
        .get(&config_url)
        .headers(headers.clone())
        .send()
        .await?;

    let config_path = format!("{}_config.json", repository);
    let mut config_file = File::create(&config_path)?;
    copy(
        &mut config_response.bytes().await?.as_ref(),
        &mut config_file,
    )?;

    // Download layers
    let layers_dir = format!("{}_layers", repository);
    fs::create_dir_all(&layers_dir)?;

    for layer in manifest["layers"]
        .as_array()
        .ok_or_else(|| anyhow!("Failed to get layers"))?
    {
        let layer_digest = layer["digest"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to get layer digest"))?;
        let layer_url = format!(
            "{}/v2/{}/{}/blobs/{}",
            registry, namespace, repository, layer_digest
        );
        let layer_response = client
            .get(&layer_url)
            .headers(headers.clone())
            .send()
            .await?;

        let layer_path =
            Path::new(&layers_dir).join(format!("{}.tar.gz", layer_digest.replace(':', "_")));
        let mut layer_file = File::create(layer_path)?;
        copy(&mut layer_response.bytes().await?.as_ref(), &mut layer_file)?;
    }

    info!(
        "Download complete. Config and layers are in {}_config.json and {}_layers/ respectively.",
        repository, repository
    );
    Ok(())
}
