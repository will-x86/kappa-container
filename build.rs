use core::panic;
#[cfg(target_os = "linux")]
fn main() {
    let filesystem =
        std::fs::read_to_string("/proc/filesystems").expect("Failed to read /proc/filesystems");
    let cgroup2_supported = filesystem.contains("cgroup2");

    if cgroup2_supported {
        println!("cargo:rustc-cfg=cgroup_v2");
        println!("cargo:rustc-cfg=feature=\"cgroup-v2\"");
    } else {
        panic!("cGroup v1 is not supported by this tool");
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    panic!("Non - linux distros are not supported")
}
