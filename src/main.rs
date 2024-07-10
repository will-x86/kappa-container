use anyhow::anyhow;
use clap::Parser;
use log::{debug, error, info};
use nix::sched::{unshare, CloneFlags};
use nix::sys::wait::waitpid;
use nix::unistd::{fork, ForkResult};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

mod cgroup_v2;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of command ( run primarily)
    cmd: String,
    /// Params of command
    params: Vec<String>,
}
fn cleanup_cgroup(cgroup_path: &Path) {
    info!("Cleaning up cgroup: {:?}", cgroup_path);
    cgroup_v2::cleanup_cgroup(cgroup_path);
    info!("Cgroup cleanup completed");
}

#[cfg(cgroup_v2)]
fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::init();

    info!("Starting container runtime");
    dotenv::dotenv().ok();
    debug!("Environment variables loaded from .env file");

    let args = Args::parse();
    info!("Parsed command line arguments: {:?}", args);

    let cgroup_path = Arc::new(PathBuf::from("/sys/fs/cgroup/container"));

    // Set up a panic hook to clean up the cgroup even if the program panics
    let cgroup_path_clone = Arc::clone(&cgroup_path);
    std::panic::set_hook(Box::new(move |_| {
        cleanup_cgroup(&cgroup_path_clone);
    }));

    match args.cmd.as_str() {
        "run" => {
            info!("Executing 'run' command");
            run(&args.params, &cgroup_path)?;
        }
        _ => {
            error!("Unimplemented command: {}", args.cmd);
            anyhow::bail!("Command not implemented");
        }
    }

    // Clean up before exiting
    cleanup_cgroup(&cgroup_path);

    info!("Container runtime completed successfully");
    Ok(())
}

fn run(params: &[String], cgroup_path: &Path) -> anyhow::Result<()> {
    info!("Starting 'run' function with params: {:?}", params);

    if params.is_empty() {
        error!("Empty parameter list provided");
        return Err(anyhow!("Cannot have param length 0"));
    }

    info!("Creating new namespaces");
    unshare(CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNS)?;
    debug!("Namespaces created successfully");

    match unsafe { fork()? } {
        ForkResult::Parent { child } => {
            info!("Parent process: waiting for child (PID: {:?})", child);
            waitpid(child, None)?;
            info!("Child process finished");
            Ok(())
        }
        ForkResult::Child => {
            info!("Child process: setting up container environment");

            debug!("Setting up control groups");
            cgroup_v2::set_cgroup(cgroup_path)?;

            debug!("Setting hostname to 'container'");
            nix::unistd::sethostname("container")?;

            info!("Changing root to Alpine filesystem");
            let alpine_path = std::env::var("ALPINE_PATH")?;
            let root = PathBuf::from(alpine_path);
            debug!("Alpine path: {:?}", root);
            nix::unistd::chroot(&root)?;
            nix::unistd::chdir(&PathBuf::from("/"))?;

            info!("Mounting /proc filesystem");
            nix::mount::mount(
                Some("proc"),
                "proc",
                Some("proc"),
                nix::mount::MsFlags::empty(),
                None::<&str>,
            )?;

            info!("Executing command: {:?}", &params[0]);
            let err = Command::new(&params[0])
                .args(&params[1..])
                .env_clear()
                .status()
                .expect("Failed to execute command");

            info!("Command execution completed, unmounting /proc");
            nix::mount::umount("/proc")?;

            let exit_code = err.code().unwrap_or(1);
            info!("Exiting child process with code: {}", exit_code);
            std::process::exit(exit_code);
        }
    }
}

#[cfg(not(cgroup_v2))]
compile_error!("Neither cgroup v1 nor cgroup v2 is supported on this system");
