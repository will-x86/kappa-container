use crate::run::cgroup_v2::set_cgroup;
use anyhow::anyhow;
use log::{debug, error, info};
use nix::sched::{unshare, CloneFlags};
use nix::sys::wait::waitpid;
use nix::unistd::{fork, ForkResult};
use std::path::{Path, PathBuf};
use std::process::Command;
pub fn run(params: &[String], img_name: &String, cgroup_path: &Path) -> anyhow::Result<()> {
    if !img_name.contains(':') {
        return Err(anyhow!(
            "Image name must include a tag (e.g., ubuntu:latest)"
        ));
    }
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
            // Split image name into name and tag
            let parts: Vec<&str> = img_name.split(':').collect();
            let root = PathBuf::from(format!("{}/{}/{}", container_path, parts[0], parts[1]));

            debug!("Attempting to chroot to: {:?}", root);
            if !root.exists() {
                return Err(anyhow!(
                    "Image {img_name} does not exist within container directory - {}, try running pull command",
                    container_path
                ));
            }

            nix::unistd::chroot(&root)?;
            debug!("Chroot successful");
            nix::unistd::chdir("/")?;
            debug!("Changed directory to root");

            info!("Mounting /proc filesystem");

            nix::mount::mount(
                Some("proc"),
                "proc",
                Some("proc"),
                nix::mount::MsFlags::empty(),
                None::<&str>,
            )?;

            info!("Executing command: {:?}", &params[0]);
            debug!("Current directory: {:?}", std::env::current_dir()?);
            debug!("Command path exists: {:?}", Path::new(&params[0]).exists());

            info!("Executing command: {:?}", &params[0]);
            debug!("Executing command with args: {:?}", &params);
            let mut child = Command::new(&params[0])
                .args(&params[1..])
                .env_clear()
                .env(
                    "PATH",
                    "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                )
                .env("TERM", "xterm")
                .env("HOME", "/root")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;

            debug!("Process spawned, reading output...");

            if let Some(stdout) = child.stdout.take() {
                use std::io::{BufRead, BufReader};
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        println!("{}", line);
                    }
                }
            }

            let status = child.wait()?;
            if !status.success() {
                error!(
                    "Command failed with exit code: {}",
                    status.code().unwrap_or(-1)
                );
                std::process::exit(status.code().unwrap_or(1));
            }
            let exit_code = status.code().unwrap_or(1);

            info!("Command execution completed, unmounting /proc");
            nix::mount::umount("/proc")?;

            info!("Exiting child process with code: {}", exit_code);
            std::process::exit(exit_code);
        }
    }
}
