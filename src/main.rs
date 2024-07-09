use anyhow::anyhow;
use clap::Parser;
use nix::sched::{unshare, CloneFlags};
use nix::sys::wait::waitpid;
use nix::unistd::{fork, ForkResult};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of command ( run primarily)
    cmd: String,
    /// Params of command
    params: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let args = Args::parse();
    match args.cmd.as_str() {
        "run" => {
            run(&args.params)?;
        }
        _ => {
            anyhow::bail!("Command not implemented");
        }
    }
    Ok(())
}

fn run(params: &[String]) -> anyhow::Result<()> {
    if params.is_empty() {
        return Err(anyhow!("Cannot have param length 0"));
    }

    // Create new namespaces
    unshare(CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNS)?;

    match unsafe { fork()? } {
        ForkResult::Parent { child } => {
            // Wait for the child to finish
            waitpid(child, None)?;
            Ok(())
        }
        ForkResult::Child => {
            // Child process
            nix::unistd::sethostname("container")?;

            // Set File system to our alpine FS
            let alpine_path = std::env::var("ALPINE_PATH")?;
            let root = PathBuf::from(alpine_path);
            nix::unistd::chroot(&root)?;
            nix::unistd::chdir(&PathBuf::from("/"))?;

            // Mount /proc
            nix::mount::mount(
                Some("proc"),
                "proc",
                Some("proc"),
                nix::mount::MsFlags::empty(),
                None::<&str>,
            )?;

            // Execute the command e.g. /bin/sh
            let err = Command::new(&params[0])
                .args(&params[1..])
                .env_clear() // Otherwise parent's env are inherited
                .status()
                .expect("Failed to execute command");
            // Unmount proc as it'll stay mounted otherweise
            nix::mount::umount("/proc")?;
            std::process::exit(err.code().unwrap_or(1));
        }
    }
}
