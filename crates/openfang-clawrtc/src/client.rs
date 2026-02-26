//! Async HTTP client for the RustChain node API.

use crate::error::{ClawRtcError, ClawRtcResult};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Default RustChain node URL.
pub const DEFAULT_NODE_URL: &str = "https://bulbous-bouffant.metalseed.net";

/// RustChain block time in seconds (10 minutes).
pub const BLOCK_TIME: u64 = 600;

/// Response from `/attest/challenge`.
#[derive(Debug, Deserialize)]
pub struct ChallengeResponse {
    pub nonce: String,
}

/// Response from `/attest/submit`.
#[derive(Debug, Deserialize)]
pub struct AttestResponse {
    pub ok: bool,
    #[serde(default)]
    pub error: Option<String>,
}

/// Response from `/epoch/enroll`.
#[derive(Debug, Deserialize)]
pub struct EnrollResponse {
    pub ok: bool,
    #[serde(default)]
    pub epoch: Option<i64>,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Response from `/health`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub ok: bool,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub uptime_s: Option<f64>,
}

/// Balance information from `/balance/{wallet}` or `/api/balance`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    #[serde(default)]
    pub balance_rtc: Option<f64>,
}

/// A miner record from `/api/miners`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerInfo {
    #[serde(default)]
    pub miner: Option<String>,
    #[serde(default)]
    pub device_arch: Option<String>,
    #[serde(default)]
    pub device_family: Option<String>,
    #[serde(default)]
    pub ts_ok: Option<i64>,
}

/// Async client for the RustChain node.
pub struct RustChainClient {
    http: reqwest::Client,
    base_url: String,
}

impl RustChainClient {
    /// Create a new client pointing at the given node URL.
    pub fn new(base_url: &str) -> Self {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true) // Self-signed certs on nodes
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Create a client using the default node URL.
    pub fn default_node() -> Self {
        Self::new(DEFAULT_NODE_URL)
    }

    /// Check node health.
    pub async fn health(&self) -> ClawRtcResult<HealthResponse> {
        let url = format!("{}/health", self.base_url);
        debug!(url, "Checking node health");
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(ClawRtcError::NodeApi(format!(
                "Health check failed: HTTP {}",
                resp.status()
            )));
        }
        Ok(resp.json().await?)
    }

    /// Get an attestation challenge nonce.
    pub async fn challenge(&self) -> ClawRtcResult<ChallengeResponse> {
        let url = format!("{}/attest/challenge", self.base_url);
        debug!(url, "Requesting attestation challenge");
        let resp = self
            .http
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ClawRtcError::NodeApi(format!(
                "Challenge failed: HTTP {status}: {body}"
            )));
        }
        Ok(resp.json().await?)
    }

    /// Submit an attestation payload.
    pub async fn submit_attestation(
        &self,
        payload: &serde_json::Value,
    ) -> ClawRtcResult<AttestResponse> {
        let url = format!("{}/attest/submit", self.base_url);
        debug!(url, "Submitting attestation");
        let resp = self.http.post(&url).json(payload).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ClawRtcError::AttestationRejected(format!(
                "HTTP {status}: {body}"
            )));
        }
        let ar: AttestResponse = resp.json().await?;
        if !ar.ok {
            return Err(ClawRtcError::AttestationRejected(
                ar.error.unwrap_or_else(|| "unknown".into()),
            ));
        }
        Ok(ar)
    }

    /// Enroll in the current epoch.
    pub async fn enroll(&self, payload: &serde_json::Value) -> ClawRtcResult<EnrollResponse> {
        let url = format!("{}/epoch/enroll", self.base_url);
        debug!(url, "Enrolling in epoch");
        let resp = self.http.post(&url).json(payload).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ClawRtcError::NodeApi(format!(
                "Enroll failed: HTTP {status}: {body}"
            )));
        }
        Ok(resp.json().await?)
    }

    /// Get wallet balance.
    pub async fn balance(&self, wallet: &str) -> ClawRtcResult<f64> {
        let url = format!("{}/api/balance?wallet={}", self.base_url, wallet);
        debug!(url, "Checking balance");
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(0.0);
        }
        let br: BalanceResponse = resp.json().await?;
        Ok(br.balance_rtc.unwrap_or(0.0))
    }

    /// List active miners.
    pub async fn miners(&self) -> ClawRtcResult<Vec<MinerInfo>> {
        let url = format!("{}/api/miners", self.base_url);
        debug!(url, "Listing miners");
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(ClawRtcError::NodeApi(format!(
                "Miners list failed: HTTP {}",
                resp.status()
            )));
        }
        Ok(resp.json().await?)
    }

    /// Submit a signed transfer.
    pub async fn transfer_signed(
        &self,
        payload: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let url = format!("{}/wallet/transfer/signed", self.base_url);
        debug!(url, "Submitting signed transfer");
        let resp = self.http.post(&url).json(payload).send().await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(ClawRtcError::NodeApi(format!(
                "Transfer failed: HTTP {status}: {}",
                body
            )));
        }
        Ok(body)
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let c = RustChainClient::default_node();
        assert_eq!(c.base_url(), DEFAULT_NODE_URL);
    }

    #[test]
    fn test_custom_url() {
        let c = RustChainClient::new("http://localhost:8099/");
        assert_eq!(c.base_url(), "http://localhost:8099");
    }
}
