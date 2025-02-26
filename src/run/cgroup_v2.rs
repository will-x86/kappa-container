use anyhow::Result;
use log::{debug, info};
use std::fs;
use std::path::Path;
use std::process;

pub fn set_cgroup(cgroup_path: &Path) -> Result<()> {
    info!("Setting up cgroup v2");

    info!("Creating new cgroup: {:?}", cgroup_path);
    fs::create_dir_all(cgroup_path)?;

    debug!("Setting memory limit to 20M");
    fs::write(cgroup_path.join("memory.max"), "20M")?;

    debug!("Setting maximum number of pids to 20");
    fs::write(cgroup_path.join("pids.max"), "20")?;

    let pid = process::id().to_string();
    debug!("Adding current process (PID: {}) to the new cgroup", pid);
    fs::write(cgroup_path.join("cgroup.procs"), pid)?;

    info!("cgroup v2 setup completed successfully");
    Ok(())
}
pub fn cleanup_cgroup(cgroup_path: &Path) -> anyhow::Result<()> {
    use std::fs;

    // Kill any remaining processes
    if let Ok(content) = fs::read_to_string(cgroup_path.join("cgroup.procs")) {
        for pid in content.split_whitespace() {
            if let Ok(pid) = pid.parse::<i32>() {
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid),
                    nix::sys::signal::Signal::SIGKILL,
                );
            }
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(100));
    fs::remove_dir(cgroup_path)?;

    Ok(())
}
