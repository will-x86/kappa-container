use std::env;
use std::fs;

fn main() {
    if env::consts::OS != "linux" {
        panic!("This program only supports Linux");
    }

    let cgroup2_mounted = is_cgroup2_mounted();
    if !cgroup2_mounted {
        panic!("This program requires cgroup v2 support");
    }
}

fn is_cgroup2_mounted() -> bool {
    let mountinfo = match fs::read_to_string("/proc/self/mountinfo") {
        Ok(contents) => contents,
        Err(_) => return false,
    };

    mountinfo.lines().any(|line| line.contains("cgroup2"))
}

