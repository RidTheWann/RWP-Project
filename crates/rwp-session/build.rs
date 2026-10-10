//! Build script for the rwp-session crate.

use std::process::Command;

fn main() {
    if let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).output()
        && output.status.success()
        && let Ok(value) = String::from_utf8(output.stdout)
    {
        let value = value.trim();
        if !value.is_empty() {
            println!("cargo::rustc-env=RWP_GIT_SHA={value}");
        }
    }
    if let Ok(target) = std::env::var("TARGET") {
        println!("cargo::rustc-env=RWP_BUILD_TARGET={target}");
    }
    if let Ok(profile) = std::env::var("PROFILE") {
        println!("cargo::rustc-env=RWP_BUILD_PROFILE={profile}");
    }
    println!("cargo::rerun-if-changed=.git/HEAD");
}
