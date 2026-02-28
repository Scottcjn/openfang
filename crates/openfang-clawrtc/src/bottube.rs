//! BoTTube video platform client.
//!
//! Provides search, trending, commenting, and voting for the BoTTube
//! AI video platform at bottube.ai.

use crate::error::{ClawRtcError, ClawRtcResult};
use tracing::debug;

const BOTTUBE_BASE: &str = "https://bottube.ai";

/// BoTTube API client.
pub struct BoTTubeClient {
    http: reqwest::Client,
    api_key: Option<String>,
}

impl BoTTubeClient {
    /// Create a new client, optionally with an API key for authenticated operations.
    pub fn new(api_key: Option<&str>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("Failed to build HTTP client");
        Self {
            http,
            api_key: api_key.map(|s| s.to_string()),
        }
    }

    /// Search videos by query string.
    pub async fn search(&self, query: &str, page: u32) -> ClawRtcResult<serde_json::Value> {
        let url = format!(
            "{}/api/search?q={}&page={}",
            BOTTUBE_BASE,
            urlencoded(query),
            page
        );
        debug!(url, "Searching BoTTube");
        let resp = self.http.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    /// Get trending videos.
    pub async fn trending(&self) -> ClawRtcResult<serde_json::Value> {
        let url = format!("{}/api/trending", BOTTUBE_BASE);
        debug!(url, "Getting BoTTube trending");
        let resp = self.http.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    /// Get platform statistics.
    pub async fn stats(&self) -> ClawRtcResult<serde_json::Value> {
        let url = format!("{}/api/stats", BOTTUBE_BASE);
        debug!(url, "Getting BoTTube stats");
        let resp = self.http.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    /// Comment on a video.
    pub async fn comment(
        &self,
        video_id: &str,
        content: &str,
        parent_id: Option<&str>,
    ) -> ClawRtcResult<serde_json::Value> {
        let key = self
            .api_key
            .as_deref()
            .ok_or_else(|| ClawRtcError::MissingApiKey("bottube".into()))?;
        let url = format!("{}/api/videos/{}/comment", BOTTUBE_BASE, video_id);
        debug!(url, video_id, "Commenting on BoTTube video");

        let mut body = serde_json::json!({
            "content": content,
            "comment_type": "comment",
        });
        if let Some(pid) = parent_id {
            body["parent_id"] = serde_json::json!(pid);
        }

        let resp = self
            .http
            .post(&url)
            .header("X-API-Key", key)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let result: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(ClawRtcError::BoTTube(format!(
                "Comment failed ({}): {}",
                status, result
            )));
        }
        Ok(result)
    }

    /// Vote on a video (1 = like, -1 = dislike, 0 = remove vote).
    pub async fn vote(
        &self,
        video_id: &str,
        vote: i8,
    ) -> ClawRtcResult<serde_json::Value> {
        let key = self
            .api_key
            .as_deref()
            .ok_or_else(|| ClawRtcError::MissingApiKey("bottube".into()))?;
        let url = format!("{}/api/videos/{}/vote", BOTTUBE_BASE, video_id);
        let action = match vote {
            1 => "like",
            -1 => "dislike",
            _ => "unvote",
        };
        debug!(url, video_id, action, "Voting on BoTTube video");

        let resp = self
            .http
            .post(&url)
            .header("X-API-Key", key)
            .json(&serde_json::json!({ "vote": vote }))
            .send()
            .await?;
        let status = resp.status();
        let result: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(ClawRtcError::BoTTube(format!(
                "Vote failed ({}): {}",
                status, result
            )));
        }
        Ok(result)
    }

    /// Get video details.
    pub async fn get_video(&self, video_id: &str) -> ClawRtcResult<serde_json::Value> {
        let url = format!("{}/api/videos/{}", BOTTUBE_BASE, video_id);
        debug!(url, "Getting BoTTube video");
        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        let result: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(ClawRtcError::BoTTube(format!(
                "Video not found ({}): {}",
                status, result
            )));
        }
        Ok(result)
    }

    /// Get comments on a video.
    pub async fn get_comments(&self, video_id: &str) -> ClawRtcResult<serde_json::Value> {
        let url = format!("{}/api/videos/{}/comments", BOTTUBE_BASE, video_id);
        debug!(url, "Getting BoTTube comments");
        let resp = self.http.get(&url).send().await?;
        Ok(resp.json().await?)
    }
}

fn urlencoded(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('+', "%2B")
        .replace('#', "%23")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let c = BoTTubeClient::new(None);
        assert!(c.api_key.is_none());
    }

    #[test]
    fn test_client_with_key() {
        let c = BoTTubeClient::new(Some("bottube_sk_test123"));
        assert_eq!(c.api_key.as_deref(), Some("bottube_sk_test123"));
    }
}
