use clap::Parser;
use log::{debug, error, info};
use std::path::PathBuf;
use std::sync::Arc;

mod pull;
mod run;
use pull::pull::pull;
use run::run::run;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of command ( run primarily)
    cmd: String,
    /// Image name
    img: String,
    /// Params of command
    params: Vec<String>,
}

#[cfg(cgroup_v2)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use run::run::cleanup_cgroup;
    env_logger::init();

    info!("Starting container runtime");
    dotenv::dotenv().ok();
    debug!("Environment variables loaded from .env file");

    let args = Args::parse();
    info!("Parsed command line arguments: {:?}", args);

    match args.cmd.as_str() {
        "run" => {
            let cgroup_path = Arc::new(PathBuf::from("/sys/fs/cgroup/container"));

            // Set up a panic hook to clean up the cgroup even if the program panics
            let cgroup_path_clone = Arc::clone(&cgroup_path);
            std::panic::set_hook(Box::new(move |_| {
                cleanup_cgroup(&cgroup_path_clone);
            }));
            info!("Executing 'run' command");
            run(&args.params, &args.img, &cgroup_path)?;

            // Clean up before exiting
            cleanup_cgroup(&cgroup_path);
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

#[cfg(not(cgroup_v2))]
compile_error!("Cgroup v2 is supported on this system");
