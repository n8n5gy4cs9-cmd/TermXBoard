use std::{path::Path, process::Command};

#[test]
fn binary_starts_and_exits_in_a_macos_pty() {
    let binary = env!("CARGO_BIN_EXE_TermXBoard");
    assert!(Path::new(binary).is_file());

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
}
