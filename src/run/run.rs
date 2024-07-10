use crate::run::cgroup_v2::{self, set_cgroup};
use anyhow::anyhow;
use log::{debug, error, info};
use nix::sched::{unshare, CloneFlags};
use nix::sys::wait::waitpid;
use nix::unistd::{fork, ForkResult};
use std::path::{Path, PathBuf};
use std::process::Command;
pub fn run(params: &[String], img_name: &String, cgroup_path: &Path) -> anyhow::Result<()> {
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
            set_cgroup(cgroup_path)?;

            debug!("Setting hostname to 'container'");
            nix::unistd::sethostname("container")?;

            let container_path = std::env::var("CONTAINER_PATH")?;
            info!("Changing root to container filesystem with path {container_path}");
            let root = PathBuf::from(format!("{}/{}", container_path, img_name));
            if !root.exists() {
                return Err(anyhow!(
                    "Image {img_name} does not exist within container directory - {}, try running pull command",
                    container_path
                ));
            }
            debug!("Container path: {:?}", root);
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

pub fn cleanup_cgroup(cgroup_path: &Path) {
    info!("Cleaning up cgroup: {:?}", cgroup_path);
    cgroup_v2::cleanup_cgroup(cgroup_path);
    info!("Cgroup cleanup completed");
}
