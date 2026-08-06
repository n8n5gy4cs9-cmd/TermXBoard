use std::{path::PathBuf, process::Command};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn scripts_explain_missing_rust_tooling() {
    for script in ["start.sh", "build.sh"] {
        let output = Command::new("/bin/bash")
            .arg(project_root().join(script))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .expect("bash should run");

        assert!(!output.status.success(), "{script} should fail");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Rust/Cargo is required"),
            "{script} should explain how to resolve the missing prerequisite"
        );
    }
}
