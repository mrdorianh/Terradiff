use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::config::Storage;
#[cfg(feature = "s3")]
use aws_sdk_s3::Client as S3Client;
#[cfg(feature = "s3")]
use aws_config::{self, BehaviorVersion};

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
        Storage::S3 { bucket, prefix } => Ok(Box::new(S3StateSource {
            bucket: bucket.clone(),
            prefix: prefix.clone(),
        })),
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

#[cfg(feature = "s3")]
struct S3StateSource {
    bucket: String,
    prefix: Option<String>,
}

#[cfg(feature = "s3")]
#[async_trait]
impl StateSource for S3StateSource {
    async fn fetch_state(&self, workspace: &str) -> Result<PathBuf> {
        let key = match &self.prefix {
            Some(p) => format!("{}/{}.tfstate", p.trim_end_matches('/'), workspace),
            None => format!("{}.tfstate", workspace),
        };

        let aws_cfg = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let client = S3Client::new(&aws_cfg);
        let resp = client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .with_context(|| format!("Fetching s3://{}/{}", self.bucket, key))?;

        // Write to temp dir
        let tmp_path = std::env::temp_dir().join(format!("{}_{}.tfstate", workspace, uuid::Uuid::new_v4()));
        let bytes = resp.body.collect().await?.into_bytes();
        tokio::fs::write(&tmp_path, &bytes).await?;
        Ok(tmp_path)
    }

    async fn list_workspaces(&self) -> Result<Vec<String>> {
        let aws_cfg = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let client = S3Client::new(&aws_cfg);

        let mut continuation_token = None;
        let mut out = Vec::new();
        loop {
            let mut req = client
                .list_objects_v2()
                .bucket(&self.bucket)
                .max_keys(1000);
            if let Some(ref token) = continuation_token {
                req = req.continuation_token(token);
            }
            if let Some(ref p) = self.prefix {
                req = req.prefix(p);
            }
            let resp = req.send().await?;
            for obj in resp.contents() {
                if let Some(key) = obj.key() {
                    if key.ends_with(".tfstate") {
                        let trimmed = if let Some(ref p) = self.prefix {
                            key.strip_prefix(&(p.to_owned() + "/")).unwrap_or(key)
                        } else {
                            key
                        };
                        if let Some(name) = trimmed.strip_suffix(".tfstate") {
                            out.push(name.to_string());
                        }
                    }
                }
            }
            if resp.is_truncated().unwrap_or(false) {
                continuation_token = resp.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }
        Ok(out)
    }
} 