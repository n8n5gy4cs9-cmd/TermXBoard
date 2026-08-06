use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn build_preserves_portable_config() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("termxboard-build-{nonce}"));
    let bin = root.join("bin");
    fs::create_dir_all(root.join("dist")).expect("dist");
    fs::create_dir_all(&bin).expect("fake bin");
    fs::copy(project_root().join("build.sh"), root.join("build.sh")).expect("build script");
    fs::write(root.join("dist/config.json"), "keep me").expect("config fixture");
    let fake_cargo = bin.join("cargo");
    fs::write(
        &fake_cargo,
        "#!/usr/bin/env bash\nmkdir -p target/release\nprintf binary > target/release/TermXBoard\n",
    )
    .expect("fake cargo");
    fs::set_permissions(&fake_cargo, fs::Permissions::from_mode(0o755)).expect("executable");

    let output = Command::new("/bin/bash")
        .arg(root.join("build.sh"))
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .output()
        .expect("build script");

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(root.join("dist/config.json")).unwrap(),
        "keep me"
    );
    assert!(root.join("dist/TermXBoard").is_file());
    let _ = fs::remove_dir_all(root);
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
