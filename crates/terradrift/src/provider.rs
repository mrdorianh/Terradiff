use std::path::PathBuf;

#[cfg(any(feature = "s3", feature = "gcs", feature = "azure"))]
use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;

use crate::config::Storage;
#[cfg(feature = "s3")]
use aws_config::{self, BehaviorVersion};
#[cfg(feature = "s3")]
use aws_sdk_s3::Client as S3Client;

#[cfg(feature = "gcs")]
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
#[cfg(feature = "gcs")]
use serde::Deserialize;

#[cfg(feature = "azure")]
use azure_storage_blobs::prelude::*;

#[cfg(feature = "azure")]
use futures_util::StreamExt;

#[cfg(any(feature = "s3", feature = "gcs"))]
use uuid::Uuid;

#[async_trait]
pub trait StateSource: Send + Sync {
    async fn fetch_state(&self, workspace: &str) -> Result<PathBuf>;

    /// List available workspaces under this source.
    async fn list_workspaces(&self) -> Result<Vec<String>>;
}

pub fn source_from_storage(storage: &Storage) -> Result<Box<dyn StateSource>> {
    match storage {
        Storage::Mock { path } => Ok(Box::new(MockStateSource { root: path.clone() })),
        #[cfg(feature = "s3")]
        Storage::S3 { bucket, prefix } => Ok(Box::new(S3StateSource {
            bucket: bucket.clone(),
            prefix: prefix.clone(),
        })),
        #[cfg(feature = "gcs")]
        Storage::Gcs { bucket, prefix } => Ok(Box::new(GcsStateSource {
            bucket: bucket.clone(),
            prefix: prefix.clone(),
        })),
        #[cfg(feature = "azure")]
        Storage::Azure { container, prefix } => Ok(Box::new(AzureStateSource {
            container: container.clone(),
            prefix: prefix.clone(),
        })),
        #[cfg(not(any(feature = "s3", feature = "gcs", feature = "azure")))]
        #[allow(unreachable_patterns)]
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
        let tmp_path =
            std::env::temp_dir().join(format!("{}_{}.tfstate", workspace, uuid::Uuid::new_v4()));
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
            let mut req = client.list_objects_v2().bucket(&self.bucket).max_keys(1000);
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

#[cfg(feature = "gcs")]
struct GcsStateSource {
    bucket: String,
    prefix: Option<String>,
}

#[cfg(feature = "gcs")]
#[derive(Deserialize)]
struct ObjectList {
    #[serde(default)]
    items: Vec<Obj>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[cfg(feature = "gcs")]
#[derive(Deserialize)]
struct Obj {
    name: String,
}

#[cfg(feature = "gcs")]
#[async_trait]
impl StateSource for GcsStateSource {
    async fn fetch_state(&self, workspace: &str) -> Result<PathBuf> {
        let obj_name = match &self.prefix {
            Some(p) => format!("{}/{}.tfstate", p.trim_end_matches('/'), workspace),
            None => format!("{}.tfstate", workspace),
        };

        let provider = gcp_auth::provider().await?;
        let token = provider
            .token(&["https://www.googleapis.com/auth/devstorage.read_only"])
            .await?;

        let encoded = percent_encode(obj_name.as_bytes(), NON_ALPHANUMERIC).to_string();
        let url = format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o/{}?alt=media",
            self.bucket, encoded
        );

        let resp = reqwest::Client::new()
            .get(&url)
            .bearer_auth(token.as_str())
            .send()
            .await
            .with_context(|| format!("Downloading gs://{}/{}", self.bucket, obj_name))?
            .error_for_status()?;

        let bytes = resp.bytes().await?;

        let tmp_path =
            std::env::temp_dir().join(format!("{}_{}.tfstate", workspace, Uuid::new_v4()));
        tokio::fs::write(&tmp_path, &bytes).await?;
        Ok(tmp_path)
    }

    async fn list_workspaces(&self) -> Result<Vec<String>> {
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;

        let provider = gcp_auth::provider().await?;
        let token = provider
            .token(&["https://www.googleapis.com/auth/devstorage.read_only"])
            .await?;
        let client = reqwest::Client::new();

        loop {
            let mut url = format!(
                "https://storage.googleapis.com/storage/v1/b/{}/o?fields=items(name),nextPageToken",
                self.bucket
            );
            if let Some(ref p) = self.prefix {
                url.push_str("&prefix=");
                url.push_str(&percent_encode(p.as_bytes(), NON_ALPHANUMERIC).to_string());
            }
            if let Some(ref token_val) = page_token {
                url.push_str("&pageToken=");
                url.push_str(token_val);
            }

            let resp = client
                .get(&url)
                .bearer_auth(token.as_str())
                .send()
                .await?
                .error_for_status()?;

            let list: ObjectList = resp.json().await?;
            for obj in list.items {
                if obj.name.ends_with(".tfstate") {
                    let trimmed = if let Some(ref p) = self.prefix {
                        obj.name
                            .strip_prefix(&(p.to_owned() + "/"))
                            .unwrap_or(&obj.name)
                    } else {
                        obj.name.as_str()
                    };
                    if let Some(stem) = trimmed.strip_suffix(".tfstate") {
                        out.push(stem.to_string());
                    }
                }
            }

            if let Some(token_val) = list.next_page_token {
                page_token = Some(token_val);
            } else {
                break;
            }
        }
        Ok(out)
    }
}

#[cfg(feature = "azure")]
struct AzureStateSource {
    container: String,
    prefix: Option<String>,
}

#[cfg(feature = "azure")]
#[async_trait]
impl StateSource for AzureStateSource {
    async fn fetch_state(&self, _workspace: &str) -> Result<PathBuf> {
        let conn = std::env::var("AZURE_STORAGE_CONNECTION_STRING")
            .context("AZURE_STORAGE_CONNECTION_STRING env var not set for Azure provider")?;

        // Parse connection string to obtain account and credentials
        let cs = azure_storage::ConnectionString::new(&conn)?;
        let credentials = cs.storage_credentials()?;
        let account = cs
            .account_name
            .ok_or_else(|| anyhow::anyhow!("AccountName missing in connection string"))?;

        let service = ClientBuilder::new(account, credentials);
        let container = service.container_client(&self.container);

        let key = match &self.prefix {
            Some(p) => format!("{}/{}.tfstate", p.trim_end_matches('/'), _workspace),
            None => format!("{}.tfstate", _workspace),
        };

        let blob = container.blob_client(&key);
        let bytes = blob
            .get_content()
            .await
            .with_context(|| format!("Downloading azure://{}/{}", self.container, key))?;

        let tmp_path =
            std::env::temp_dir().join(format!("{}_{}.tfstate", _workspace, Uuid::new_v4()));
        tokio::fs::write(&tmp_path, bytes).await?;
        Ok(tmp_path)
    }
    async fn list_workspaces(&self) -> Result<Vec<String>> {
        let conn = std::env::var("AZURE_STORAGE_CONNECTION_STRING")
            .context("AZURE_STORAGE_CONNECTION_STRING env var not set for Azure provider")?;

        let cs = azure_storage::ConnectionString::new(&conn)?;
        let credentials = cs.storage_credentials()?;
        let account = cs
            .account_name
            .ok_or_else(|| anyhow::anyhow!("AccountName missing in connection string"))?;

        let service = ClientBuilder::new(account, credentials);
        let container = service.container_client(&self.container);

        let mut builder = container.list_blobs();
        if let Some(ref p) = self.prefix {
            builder = builder.prefix(p.clone());
        }

        let mut stream = Box::pin(builder.into_stream());
        let prefix_str = self.prefix.as_deref().unwrap_or("");
        let mut out = Vec::new();
        while let Some(resp) = stream.next().await {
            let page = resp?;
            for blob in page.blobs.blobs() {
                // `blob` is of type `&azure_storage_blobs::container::operations::list_blobs::Blob`
                if blob.name.ends_with(".tfstate") {
                    let trimmed = blob.name.strip_prefix(prefix_str).unwrap_or(&blob.name);
                    if let Some(stem) = trimmed.strip_suffix(".tfstate") {
                        let name = stem.trim_start_matches('/');
                        out.push(name.to_string());
                    }
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn mock_source_lists_workspaces() {
        let dir = tempdir().unwrap();
        // Create sample tfstate files
        tokio::fs::write(dir.path().join("ws1.tfstate"), b"{}")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("ws2.tfstate"), b"{}")
            .await
            .unwrap();

        let storage = Storage::Mock {
            path: dir.path().to_path_buf(),
        };
        let src = source_from_storage(&storage).unwrap();
        let list = src.list_workspaces().await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&"ws1".to_string()));
        assert!(list.contains(&"ws2".to_string()));
    }
}
