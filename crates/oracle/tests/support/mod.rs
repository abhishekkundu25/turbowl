//! Helpers the test binaries share. Each test binary uses part of this module.
#![allow(dead_code)]

pub mod data;

/// Where a test writes its report: `target/oracle/<name>.md`.
pub fn report(name: &str, text: &str) -> std::path::PathBuf {
    let path = turbowl_oracle::workspace_root()
        .join("target/oracle")
        .join(format!("{name}.md"));
    std::fs::create_dir_all(path.parent().expect("has a parent")).expect("target/oracle is writable");
    std::fs::write(&path, text).expect("report is writable");
    path
}
