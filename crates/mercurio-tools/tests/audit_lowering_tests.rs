#![cfg(feature = "legacy-pilot-tools")]

use std::path::PathBuf;
use std::process::Command;

#[test]
fn audit_reports_semantic_defaults_backed_by_lowering_rules() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();

    let output = Command::new(env!("CARGO_BIN_EXE_audit_lowering"))
        .current_dir(workspace_root)
        .arg("--min-reviewed-rules")
        .arg("99")
        .output()
        .expect("run audit_lowering");

    assert!(
        output.status.success(),
        "audit_lowering failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("semantic default constructs without declarative lowering rules: 0"),
        "missing semantic-default/lowering-rule bridge metric\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("remaining policies: 0"),
        "missing hard-coded lowering policy burndown baseline\nstdout:\n{stdout}"
    );
}
