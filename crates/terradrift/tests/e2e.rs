use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::{tempdir, NamedTempFile};
use std::io::Write;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn drift_exit_code() {
    // Temp dir for mock tfstate files
    let state_dir = tempdir().unwrap();
    fs::write(state_dir.path().join("ws_clean.tfstate"), b"{}" ).unwrap();
    fs::write(state_dir.path().join("ws_drift.tfstate"), b"{}" ).unwrap();

    // Fake terraform binary that simulates drift
    let bin_dir = tempdir().unwrap();
    let bin_path = bin_dir.path().join("terraform");
    let mut script = fs::File::create(&bin_path).unwrap();
    writeln!(script, "#!/usr/bin/env bash").unwrap();
    writeln!(script, "if [[ \"$1\" == \"version\" ]]; then").unwrap();
    writeln!(script, "echo '{{\"terraform_version\":\"1.7.5\"}}'").unwrap();
    writeln!(script, "exit 0; fi").unwrap();

    // For any plan command, emit JSON with a single changed resource and exit 2
    writeln!(script, "echo '{{\"resource_changes\":[{{\"change\":{{\"actions\":[\"update\"]}}}}]}}'").unwrap();
    writeln!(script, "exit 2").unwrap();
    drop(script);
    let mut perms = fs::metadata(&bin_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&bin_path, perms).unwrap();

    // Build temporary terradrift config pointing at mock provider
    let toml_content = format!(
        r#"[profiles.prod.storage]
provider = "mock"
path = "{}"
"#, state_dir.path().display());
    let toml_file = NamedTempFile::new().unwrap();
    fs::write(toml_file.path(), toml_content).unwrap();

    // Invoke CLI
    let mut cmd = Command::cargo_bin("terradrift").unwrap();
    cmd.arg("diff")
        .arg("-p").arg("prod")
        .arg("--config").arg(toml_file.path())
        .env("PATH", format!("{}:{}", bin_dir.path().display(), std::env::var("PATH").unwrap_or_default()))
        .env("TERRADRIFT_TF_CACHE", tempdir().unwrap().path());

    cmd.assert()
        .failure() // exit code != 0 expected (drift yields 2)
        .code(predicate::eq(2))
        .stdout(predicate::str::contains("\"drift\": true"));
} 