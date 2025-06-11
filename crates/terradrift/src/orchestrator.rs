use std::sync::Arc;

use anyhow::Result;
use futures::future::join_all;
use tokio::sync::Semaphore;

use crate::config::Profile;
use crate::provider::source_from_storage;
use crate::terraform::{detect_drift, ensure_terraform};

#[derive(Debug, serde::Serialize)]
pub struct WorkspaceResult {
    pub workspace: String,
    pub drift: bool,
    pub changed_resources: u64,
    pub duration_ms: u128,
}

pub async fn run_profile(_profile_name: &str, profile: &Profile, jobs: Option<usize>) -> Result<Vec<WorkspaceResult>> {
    let source = source_from_storage(&profile.storage)?;
    let workspaces = source.list_workspaces().await?;

    let limit = jobs.unwrap_or_else(|| num_cpus::get().max(2));
    let sem = Arc::new(Semaphore::new(limit));
    let bin = ensure_terraform(None).await?;

    let mut handles = Vec::new();

    for ws in workspaces {
        let permit = sem.clone().acquire_owned().await?;
        let src = source_from_storage(&profile.storage)?; // new boxed instance
        let bin_path = bin.clone();
        let ws_name = ws.clone();
        handles.push(tokio::spawn(async move {
            let _p = permit;
            let _state = src.fetch_state(&ws_name).await?; // not used yet
            let report = detect_drift(&bin_path, &_state).await?;
            Ok::<_, anyhow::Error>(WorkspaceResult {
                workspace: ws_name,
                drift: report.drift,
                changed_resources: report.changed_resources,
                duration_ms: report.duration_ms,
            })
        }));
    }

    let mut results = Vec::new();
    for res in join_all(handles).await {
        results.push(res??);
    }

    Ok(results)
} 