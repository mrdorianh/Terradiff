use anyhow::{Context, Result};
use serde_json;

pub async fn post_slack(webhook_url: &str, text: &str) -> Result<()> {
    let payload = serde_json::json!({"text": text});
    reqwest::Client::new()
        .post(webhook_url)
        .json(&payload)
        .send()
        .await
        .with_context(|| "Sending Slack webhook")?
        .error_for_status()
        .with_context(|| "Slack API error")?;
    Ok(())
}
