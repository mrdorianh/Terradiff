use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::config::Storage;

#[async_trait]
pub trait StateSource: Send + Sync {
    async fn fetch_state(&self, workspace: &str) -> Result<PathBuf>;

    /// List available workspaces under this source.
    async fn list_workspaces(&self) -> Result<Vec<String>>;
}

pub fn source_from_storage(storage: &Storage) -> Result<Box<dyn StateSource>> {
    match storage {
        Storage::Mock { path } => Ok(Box::new(MockStateSource {
            root: path.clone(),
        })),
        // TODO: other providers
        _ => anyhow::bail!("Provider not yet implemented"),
    }
}

struct MockStateSource {
    root: PathBuf,
}

#[async_trait]
impl StateSource for MockStateSource {
    async fn fetch_state(&self, workspace: &str) -> Result<PathBuf> {
        let file_path = self.root.join(format!("{workspace}.tfstate"));
        if file_path.exists() {
            Ok(file_path)
        } else {
            Err(anyhow::anyhow!(
                "Mock tfstate file not found: {}",
                file_path.display()
            ))
        }
    }

    async fn list_workspaces(&self) -> Result<Vec<String>> {
        let mut list = Vec::new();
        let entries = tokio::fs::read_dir(&self.root).await?;
        tokio::pin!(entries);
        use tokio_stream::StreamExt;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "tfstate" {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        list.push(stem.to_string());
                    }
                }
            }
        }
        Ok(list)
    }
} 