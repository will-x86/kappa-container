use anyhow::Result;
use log::{debug, error, info};
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
pub fn cleanup_cgroup(cgroup_path: &Path) {
    if let Err(e) = fs::remove_dir(cgroup_path) {
        error!("Failed to remove cgroup: {}", e);
    }
}
