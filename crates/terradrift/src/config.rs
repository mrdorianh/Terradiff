use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_CONFIG_NAME: &str = "terradrift.toml";

#[derive(Debug, Deserialize)]
pub struct Config {
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub storage: Storage,
    /// Optional workspace-specific concurrency override
    pub jobs: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum Storage {
    Mock {
        path: PathBuf,
    },
    #[cfg(feature = "s3")]
    S3 {
        bucket: String,
        prefix: Option<String>,
    },
    #[cfg(feature = "gcs")]
    Gcs {
        bucket: String,
        prefix: Option<String>,
    },
    #[cfg(feature = "azure")]
    Azure {
        container: String,
        prefix: Option<String>,
    },
}

impl Config {
    /// Load configuration from an explicit path, or search upward from current dir.
    pub fn load(path_override: Option<PathBuf>) -> Result<Self> {
        let path = match path_override {
            Some(p) => p,
            None => find_upwards(DEFAULT_CONFIG_NAME)
                .context("Failed to locate terradrift.toml in current or parent directories")?,
        };

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Reading config file {}", path.display()))?;
        let cfg: Config = toml::from_str(&contents)
            .with_context(|| format!("Parsing TOML config {}", path.display()))?;
        Ok(cfg)
    }

    pub fn profile(&self, name: &str) -> Result<&Profile> {
        self.profiles
            .get(name)
            .with_context(|| format!("Profile '{}' not found in config", name))
    }
}

fn find_upwards(file_name: &str) -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(file_name);
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn load_config_success() {
        let dir = tempfile::tempdir().unwrap();
        let mock_path = dir.path().to_path_buf();

        // Compose minimal TOML configuration
        let toml = format!(
            r#"[profiles.prod.storage]
provider = "mock"
path = "{}"
"#,
            mock_path.display()
        );

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(toml.as_bytes()).unwrap();

        let cfg = Config::load(Some(file.path().to_path_buf())).unwrap();
        let profile = cfg.profile("prod").unwrap();
        match &profile.storage {
            Storage::Mock { path } => assert_eq!(path, &mock_path),
            #[cfg(any(feature = "s3", feature = "gcs", feature = "azure"))]
            _ => panic!("unexpected storage type"),
        }
    }

    #[test]
    fn missing_profile_errors() {
        let toml = r#"[profiles.dev.storage]
provider = "mock"
path = "/tmp"
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(toml.as_bytes()).unwrap();
        let cfg = Config::load(Some(file.path().to_path_buf())).unwrap();
        let result = cfg.profile("does_not_exist");
        assert!(result.is_err());
    }
}
