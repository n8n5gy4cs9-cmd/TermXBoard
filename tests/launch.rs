use std::{
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn binary_starts_and_exits_in_a_macos_pty() {
    let binary = env!("CARGO_BIN_EXE_TermXBoard");
    assert!(Path::new(binary).is_file());
    let config = Path::new(binary)
        .parent()
        .expect("binary directory")
        .join("config.json");
    let previous_config = fs::read(&config).ok();
    fs::write(
        &config,
        r#"{"city":"Turku, Finland","theme":"signature-neon","reduced_motion":false}"#,
    )
    .expect("launch fixture config");

    let command = format!("printf q | /usr/bin/script -q /dev/null '{}'", binary);
    let output = Command::new("/bin/bash")
        .args(["-c", &command])
        .output()
        .expect("macOS script utility should launch the binary in a PTY");

    assert!(
        output.status.success(),
        "TermXBoard failed to start and quit: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let project_dir = std::env::temp_dir().join(format!("termxboard-launch-{nonce}"));
    fs::create_dir_all(&project_dir).expect("project fixture directory");
    fs::write(
        project_dir.join("progress.json"),
        r#"{"project":"Launch fixture","tasks":[{"id":"T-1","status":"todo","dependsOn":[]}]}"#,
    )
    .expect("progress fixture");
    let launch_with_project = format!(
        "printf q | /usr/bin/script -q /dev/null '{}' progress.json",
        binary
    );
    let project_output = Command::new("/bin/bash")
        .args(["-c", &launch_with_project])
        .current_dir(&project_dir)
        .output()
        .expect("launch with relative Progress File argument");
    assert!(project_output.status.success());
    let saved_config: serde_json::Value =
        serde_json::from_slice(&fs::read(&config).expect("config saved after project load"))
            .expect("saved config JSON");
    assert_eq!(
        saved_config["remembered_project"],
        fs::canonicalize(project_dir.join("progress.json"))
            .expect("canonical fixture")
            .to_string_lossy()
            .as_ref()
    );

    let absolute_argument =
        fs::canonicalize(project_dir.join("progress.json")).expect("absolute Progress File path");
    let launch_absolute = format!(
        "printf q | /usr/bin/script -q /dev/null '{}' '{}'",
        binary,
        absolute_argument.display()
    );
    let absolute_output = Command::new("/bin/bash")
        .args(["-c", &launch_absolute])
        .output()
        .expect("launch with absolute Progress File argument");
    assert!(absolute_output.status.success());
    let _ = fs::remove_dir_all(project_dir);

    if let Some(contents) = previous_config {
        fs::write(config, contents).expect("restore config");
    } else {
        let _ = fs::remove_file(config);
    }
}
