use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct ApprovalRequest<'a> {
    device_id: &'a str,
    action: &'a str,
    summary: &'a str,
    cost_usd: f64,
    nonce: &'a str,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Approved,
    Denied,
    Pending,
}

#[derive(Debug, Deserialize)]
struct ApprovalResponse {
    status: ApprovalStatus,
    #[allow(dead_code)]
    reason: Option<String>,
}

pub struct ApprovalClient {
    base_url: String,
    device_id: String,
    http: reqwest::Client,
}

impl ApprovalClient {
    pub fn new(base_url: impl Into<String>, device_id: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            device_id: device_id.into(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(90))
                .build()
                .expect("reqwest client"),
        }
    }

    pub async fn request(&self, action: &str, summary: &str, cost_usd: f64) -> Result<ApprovalStatus> {
        let nonce = uuid::Uuid::new_v4().to_string();
        let resp = self
            .http
            .post(format!("{}/oob/request", self.base_url))
            .json(&ApprovalRequest {
                device_id: &self.device_id,
                action,
                summary,
                cost_usd,
                nonce: &nonce,
            })
            .send()
            .await
            .context("oob request send")?
            .error_for_status()
            .context("oob request status")?
            .json::<ApprovalResponse>()
            .await
            .context("oob request decode")?;
        Ok(resp.status)
    }
}
