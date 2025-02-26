use clap::Parser;
use log::{debug, error, info};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;

mod pull;
mod run;
use crate::run::cgroup_v2::cleanup_cgroup;
use crate::run::run::run;
use pull::pull::pull;
#[derive(Parser, Debug)]
#[command(
    name = "kappa-container",
    about = "A lightweight container runtime written in Rust",
    version,
    author,
    long_about = "A container runtime that supports basic container operations like running and pulling images. \
                  Requires Linux with cgroup v2 support.

Examples:
  # Run a shell in an Ubuntu container:
  kappa-container run ubuntu:latest /bin/bash

  # Pull an image:
  kappa-container pull nginx:latest

  # Run a web server:
  kappa-container run nginx:latest nginx -g 'daemon off;'"
)]
#[command(propagate_version = true)]
struct Args {
    #[arg(value_parser)]
    #[arg(help = "Available commands: 'run' (execute a container), 'pull' (download an image)")]
    cmd: String,

    #[arg(help = "Image name in format: [registry/]image[:tag]")]
    #[arg(value_parser)]
    img: String,

    #[arg(help = "Command and arguments to run inside the container")]
    #[arg(trailing_var_arg = true)]
    #[arg(value_parser)]
    params: Vec<String>,
}

#[cfg(not(target_os = "linux"))]
compile_error!("This program only supports Linux");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write;
            writeln!(
                buf,
                "[{}] [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    info!("Starting kappa-container");

    // Load environment variables
    dotenv::dotenv().ok();

    // Set default container path if not set
    if std::env::var("CONTAINER_PATH").is_err() {
        std::env::set_var("CONTAINER_PATH", "/var/lib/kappa-container/images");
    }

    // Ensure container path exists
    let container_path = std::env::var("CONTAINER_PATH")?;
    std::fs::create_dir_all(&container_path)?;
    debug!("Environment variables loaded from .env file");

    let args = Args::parse();
    info!("Parsed command line arguments: {:?}", args);

    match args.cmd.as_str() {
        "run" => {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let cgroup_path =
                Arc::new(PathBuf::from(format!("/sys/fs/cgroup/kappa-{}", timestamp)));

            let cgroup_path_clone = Arc::clone(&cgroup_path);
            std::panic::set_hook(Box::new(move |_| {
                cleanup_cgroup(&cgroup_path_clone).unwrap();
            }));

            let cgroup_path_clone = Arc::clone(&cgroup_path);
            tokio::spawn(async move {
                if let Ok(()) = signal::ctrl_c().await {
                    info!("Received Ctrl+C, cleaning up...");
                    cleanup_cgroup(&cgroup_path_clone).unwrap();
                    std::process::exit(0);
                }
            });

            info!("Executing 'run' command");
            run(&args.params, &args.img, &cgroup_path)?;

            cleanup_cgroup(&cgroup_path)?;
            info!("Container runtime completed successfully");
        }
        "pull" => {
            pull(&args.img).await?;
        }
        _ => {
            error!("Unimplemented command: {}", args.cmd);
            anyhow::bail!("Command not implemented");
        }
    }

    Ok(())
}
