use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;

use tokio::process::Command;
use std::io::Read;
use anyhow::Result;
use tokio::io::{AsyncReadExt};
use serde_json::{Value};

const DEFAULT_TERRAFORM_VERSION: &str = "1.7.5";

pub struct DriftReport {
    pub changed_resources: u64,
    pub drift: bool,
    pub duration_ms: u128,
    pub terraform_version: String,
}

/// Ensure terraform binary for given version is present and executable.
/// Returns path to binary.
pub async fn ensure_terraform(version: Option<&str>) -> Result<PathBuf> {
    // Check PATH first
    if let Ok(bin) = which::which("terraform") {
        return Ok(bin);
    }

    let version = version.unwrap_or(DEFAULT_TERRAFORM_VERSION);
    let cache_root = std::env::var("TERRADRIFT_TF_CACHE").unwrap_or_else(|_| {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("terradrift/terraform")
            .to_string_lossy()
            .to_string()
    });
    let bin_path = Path::new(&cache_root)
        .join(version)
        .join("terraform");
    if bin_path.exists() {
        return Ok(bin_path);
    }
    fs::create_dir_all(bin_path.parent().unwrap())?;

    download_terraform(version, &bin_path).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms)?;
    }

    Ok(bin_path)
}

async fn download_terraform(version: &str, dst_bin: &Path) -> Result<()> {
    let (os, arch) = platform_triple();
    let file_name = format!("terraform_{version}_{os}_{arch}.zip");
    let url = format!("https://releases.hashicorp.com/terraform/{version}/{file_name}");

    let resp = reqwest::get(&url).await?.error_for_status()?;
    let bytes = resp.bytes().await?;

    let dst_path = dst_bin.to_path_buf();

    // Offload decompression to blocking thread
    tokio::task::spawn_blocking(move || -> Result<()> {
        let reader = std::io::Cursor::new(bytes);
        let mut zip = zip::ZipArchive::new(reader)?;
        let mut file = zip.by_name("terraform")?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        std::fs::write(&dst_path, &buffer)?;
        Ok(())
    })
    .await??;

    Ok(())
}

/// Detect host OS/ARCH to construct official terraform package names.
fn platform_triple() -> (&'static str, &'static str) {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "darwin",
        other => other, // unsupported may fail later
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    };
    (os, arch)
}

/// Run `terraform version -json` to capture version string
pub async fn terraform_version(bin: &Path) -> Result<String> {
    let output = Command::new(bin)
        .arg("version")
        .arg("-json")
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .await?;
    let v: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    Ok(v["terraform_version"].as_str().unwrap_or_default().to_string())
}

/// Stub drift detection – just runs `terraform version` for now.
pub async fn detect_drift_stub(bin: &Path) -> Result<DriftReport> {
    detect_drift(bin, Path::new("/dev/null")).await
}

pub async fn detect_drift(bin: &Path, state_path: &Path) -> Result<DriftReport> {
    let start = Instant::now();

    // Spawn terraform plan. For now we rely on exit code 0/2; capture JSON on stdout.
    let mut cmd = Command::new(bin);
    cmd.arg("plan")
        .arg("-detailed-exitcode")
        .arg("-input=false")
        .arg("-no-color")
        .arg("-refresh=true")
        .arg("-json");

    // If user supplied local state file path we inject env var so plan picks it up via backend override.
    // Many backends ignore local -state flag, but this is acceptable for the initial implementation.
    // We set TF_STATE environment variable for consistency with tests.
    cmd.env("TF_STATE", state_path);

    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    let mut stdout = child.stdout.take().expect("child stdout");

    // Read all bytes for now. TODO(zio): Replace with streaming parser to cap memory <150 MB.
    let mut buf = Vec::new();
    stdout.read_to_end(&mut buf).await?;

    let status = child.wait().await?;

    // Parse JSON plan (might be empty on errors)
    let mut changed = 0u64;
    if !buf.is_empty() {
        if let Ok(v) = serde_json::from_slice::<Value>(&buf) {
            if let Some(arr) = v.get("resource_changes").and_then(|v| v.as_array()) {
                for rc in arr {
                    if let Some(actions) = rc
                        .get("change")
                        .and_then(|c| c.get("actions"))
                        .and_then(|a| a.as_array())
                    {
                        let only_noop = actions.len() == 1
                            && actions[0].as_str().unwrap_or("") == "no-op";
                        if !only_noop {
                            changed += 1;
                        }
                    }
                }
            }
        }
    }

    let tf_version = terraform_version(bin).await.unwrap_or_default();

    Ok(DriftReport {
        changed_resources: changed,
        drift: status.code() == Some(2) || changed > 0,
        duration_ms: start.elapsed().as_millis(),
        terraform_version: tf_version,
    })
} 