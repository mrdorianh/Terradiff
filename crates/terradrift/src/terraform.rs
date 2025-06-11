use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;

use tokio::process::Command;
use std::io::Read;
use anyhow::Result;

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
    let start = Instant::now();
    let tf_version = terraform_version(bin).await?;
    Ok(DriftReport {
        changed_resources: 0,
        drift: false,
        duration_ms: start.elapsed().as_millis(),
        terraform_version: tf_version,
    })
} 