//! Multi-platform content discovery and posting via Grazer.
//!
//! Supports: BoTTube, Moltbook, 4claw, ClawHub, PinchedIn, AgentChan,
//! ClawSta, ClawNews, ClawTasks, ClawCities, SwarmHub, Agent Directory.

use crate::error::{ClawRtcError, ClawRtcResult};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Platform identifiers for Grazer operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Bottube,
    Moltbook,
    #[serde(rename = "4claw")]
    FourClaw,
    Clawhub,
    Pinchedin,
    Agentchan,
    Clawsta,
    Clawnews,
    Clawtasks,
    Clawcities,
    Swarmhub,
    Directory,
}

impl std::str::FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bottube" => Ok(Self::Bottube),
            "moltbook" => Ok(Self::Moltbook),
            "4claw" | "fourclaw" => Ok(Self::FourClaw),
            "clawhub" => Ok(Self::Clawhub),
            "pinchedin" => Ok(Self::Pinchedin),
            "agentchan" => Ok(Self::Agentchan),
            "clawsta" => Ok(Self::Clawsta),
            "clawnews" => Ok(Self::Clawnews),
            "clawtasks" => Ok(Self::Clawtasks),
            "clawcities" => Ok(Self::Clawcities),
            "swarmhub" => Ok(Self::Swarmhub),
            "directory" => Ok(Self::Directory),
            _ => Err(format!("Unknown platform: {s}")),
        }
    }
}

impl Platform {

    pub fn base_url(&self) -> &'static str {
        match self {
            Self::Bottube => "https://bottube.ai",
            Self::Moltbook => "https://www.moltbook.com",
            Self::FourClaw => "https://www.4claw.org",
            Self::Clawhub => "https://clawhub.ai",
            Self::Pinchedin => "https://www.pinchedin.com",
            Self::Agentchan => "https://chan.alphakek.ai",
            Self::Clawsta => "https://clawsta.io",
            Self::Clawnews => "https://clawnews.io",
            Self::Clawtasks => "https://clawtasks.com",
            Self::Clawcities => "https://clawcities.com",
            Self::Swarmhub => "https://swarmhub.onrender.com",
            Self::Directory => "https://directory.ctxly.app",
        }
    }

    pub fn all_names() -> &'static [&'static str] {
        &[
            "bottube",
            "moltbook",
            "4claw",
            "clawhub",
            "pinchedin",
            "agentchan",
            "clawsta",
            "clawnews",
            "clawtasks",
            "clawcities",
            "swarmhub",
            "directory",
        ]
    }
}

/// Multi-platform Grazer client.
pub struct GrazerClient {
    http: reqwest::Client,
}

impl Default for GrazerClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GrazerClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("Failed to build HTTP client");
        Self { http }
    }

    /// Discover content on a platform.
    pub async fn discover(
        &self,
        platform: Platform,
        api_key: Option<&str>,
        limit: u32,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        match platform {
            Platform::Bottube => self.discover_bottube(limit, extra).await,
            Platform::Moltbook => self.discover_moltbook(api_key, limit, extra).await,
            Platform::FourClaw => self.discover_fourclaw(api_key, limit, extra).await,
            Platform::Clawhub => self.discover_clawhub(limit, extra).await,
            Platform::Pinchedin => self.discover_pinchedin(api_key, limit).await,
            Platform::Agentchan => self.discover_agentchan(limit, extra).await,
            Platform::Clawsta => self.discover_clawsta(api_key, limit).await,
            Platform::Clawnews => self.discover_clawnews(api_key, limit).await,
            Platform::Clawtasks => self.discover_clawtasks(api_key, limit).await,
            Platform::Swarmhub => self.discover_swarmhub(limit).await,
            Platform::Directory => self.discover_directory(limit, extra).await,
            Platform::Clawcities => Ok(serde_json::json!({
                "platform": "clawcities",
                "note": "ClawCities is a personal website platform. Use grazer_post to comment on sites."
            })),
        }
    }

    /// Post content to a platform.
    pub async fn post(
        &self,
        platform: Platform,
        api_key: &str,
        title: &str,
        content: &str,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        match platform {
            Platform::Moltbook => self.post_moltbook(api_key, title, content, extra).await,
            Platform::FourClaw => self.post_fourclaw(api_key, title, content, extra).await,
            Platform::Agentchan => self.post_agentchan(api_key, content, extra).await,
            Platform::Clawsta => self.post_clawsta(api_key, content).await,
            Platform::Clawnews => self.post_clawnews(api_key, title, content, extra).await,
            Platform::Pinchedin => self.post_pinchedin(api_key, content).await,
            Platform::Clawtasks => self.post_clawtask(api_key, title, content, extra).await,
            _ => Err(ClawRtcError::Grazer(format!(
                "Posting not supported for platform: {:?}",
                platform
            ))),
        }
    }

    /// Search ClawHub skills.
    pub async fn search_clawhub(
        &self,
        query: &str,
        limit: u32,
    ) -> ClawRtcResult<serde_json::Value> {
        let url = format!(
            "{}/api/v1/skills?search={}&limit={}",
            Platform::Clawhub.base_url(),
            urlencoded(query),
            limit
        );
        debug!(url, "Searching ClawHub");
        let resp = self.http.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    // ─── Platform-specific discover implementations ─────────────────────

    async fn discover_bottube(
        &self,
        limit: u32,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let category = extra["category"].as_str().unwrap_or("");
        let agent = extra["agent"].as_str().unwrap_or("");
        let mut url = format!("{}/api/videos?limit={}", Platform::Bottube.base_url(), limit);
        if !category.is_empty() {
            url.push_str(&format!("&category={}", urlencoded(category)));
        }
        if !agent.is_empty() {
            url.push_str(&format!("&agent={}", urlencoded(agent)));
        }
        debug!(url, "Discovering BoTTube");
        let resp = self.http.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    async fn discover_moltbook(
        &self,
        api_key: Option<&str>,
        limit: u32,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let submolt = extra["submolt"].as_str().unwrap_or("tech");
        let url = format!(
            "{}/api/v1/posts?submolt={}&limit={}",
            Platform::Moltbook.base_url(),
            urlencoded(submolt),
            limit
        );
        debug!(url, "Discovering Moltbook");
        let mut req = self.http.get(&url);
        if let Some(key) = api_key {
            req = req.bearer_auth(key);
        }
        let resp = req.send().await?;
        Ok(resp.json().await?)
    }

    async fn discover_fourclaw(
        &self,
        api_key: Option<&str>,
        limit: u32,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let board = extra["board"].as_str().unwrap_or("b");
        let url = format!(
            "{}/api/v1/boards/{}/threads?limit={}",
            Platform::FourClaw.base_url(),
            urlencoded(board),
            limit.min(20)
        );
        debug!(url, "Discovering 4claw");
        let mut req = self.http.get(&url);
        if let Some(key) = api_key {
            req = req.bearer_auth(key);
        }
        let resp = req.send().await?;
        Ok(resp.json().await?)
    }

    async fn discover_clawhub(
        &self,
        limit: u32,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let search = extra["search"].as_str().unwrap_or("");
        let mut url = format!(
            "{}/api/v1/skills?limit={}",
            Platform::Clawhub.base_url(),
            limit
        );
        if !search.is_empty() {
            url.push_str(&format!("&search={}", urlencoded(search)));
        }
        debug!(url, "Discovering ClawHub");
        let resp = self.http.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    async fn discover_pinchedin(
        &self,
        api_key: Option<&str>,
        limit: u32,
    ) -> ClawRtcResult<serde_json::Value> {
        let key = api_key.ok_or_else(|| ClawRtcError::MissingApiKey("pinchedin".into()))?;
        let url = format!(
            "{}/api/feed?limit={}",
            Platform::Pinchedin.base_url(),
            limit
        );
        debug!(url, "Discovering PinchedIn");
        let resp = self
            .http
            .get(&url)
            .bearer_auth(key)
            .header("Content-Type", "application/json")
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    async fn discover_agentchan(
        &self,
        limit: u32,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let board = extra["board"].as_str().unwrap_or("ai");
        let url = format!(
            "{}/api/boards/{}/catalog",
            Platform::Agentchan.base_url(),
            urlencoded(board)
        );
        debug!(url, "Discovering AgentChan");
        let resp = self.http.get(&url).send().await?;
        let mut data: serde_json::Value = resp.json().await?;
        // Trim to limit
        if let Some(arr) = data.get_mut("data").and_then(|d| d.as_array_mut()) {
            arr.truncate(limit as usize);
        }
        Ok(data)
    }

    async fn discover_clawsta(
        &self,
        api_key: Option<&str>,
        limit: u32,
    ) -> ClawRtcResult<serde_json::Value> {
        let url = format!("{}/v1/posts?limit={}", Platform::Clawsta.base_url(), limit);
        debug!(url, "Discovering ClawSta");
        let mut req = self.http.get(&url);
        if let Some(key) = api_key {
            req = req.bearer_auth(key);
        }
        let resp = req.send().await?;
        Ok(resp.json().await?)
    }

    async fn discover_clawnews(
        &self,
        api_key: Option<&str>,
        limit: u32,
    ) -> ClawRtcResult<serde_json::Value> {
        let url = format!(
            "{}/api/stories?limit={}",
            Platform::Clawnews.base_url(),
            limit
        );
        debug!(url, "Discovering ClawNews");
        let mut req = self.http.get(&url);
        if let Some(key) = api_key {
            req = req.bearer_auth(key);
        }
        let resp = req.send().await?;
        Ok(resp.json().await?)
    }

    async fn discover_clawtasks(
        &self,
        api_key: Option<&str>,
        limit: u32,
    ) -> ClawRtcResult<serde_json::Value> {
        let key = api_key.ok_or_else(|| ClawRtcError::MissingApiKey("clawtasks".into()))?;
        let url = format!(
            "{}/api/bounties?status=open&limit={}",
            Platform::Clawtasks.base_url(),
            limit
        );
        debug!(url, "Discovering ClawTasks");
        let resp = self
            .http
            .get(&url)
            .bearer_auth(key)
            .header("Content-Type", "application/json")
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    async fn discover_swarmhub(&self, limit: u32) -> ClawRtcResult<serde_json::Value> {
        let url = format!("{}/api/v1/agents", Platform::Swarmhub.base_url());
        debug!(url, "Discovering SwarmHub");
        let resp = self.http.get(&url).send().await?;
        let mut data: serde_json::Value = resp.json().await?;
        if let Some(arr) = data.get_mut("agents").and_then(|a| a.as_array_mut()) {
            arr.truncate(limit as usize);
        }
        Ok(data)
    }

    async fn discover_directory(
        &self,
        limit: u32,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let category = extra["category"].as_str().unwrap_or("");
        let mut url = format!(
            "{}/api/services?limit={}",
            Platform::Directory.base_url(),
            limit
        );
        if !category.is_empty() {
            url.push_str(&format!("&category={}", urlencoded(category)));
        }
        debug!(url, "Discovering Agent Directory");
        let resp = self.http.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    // ─── Platform-specific post implementations ─────────────────────────

    async fn post_moltbook(
        &self,
        api_key: &str,
        title: &str,
        content: &str,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let submolt = extra["submolt"].as_str().unwrap_or("general");
        let url = format!("{}/api/v1/posts", Platform::Moltbook.base_url());
        debug!(url, submolt, "Posting to Moltbook");
        let resp = self
            .http
            .post(&url)
            .bearer_auth(api_key)
            .json(&serde_json::json!({
                "title": title,
                "content": content,
                "submolt_name": submolt,
            }))
            .send()
            .await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(ClawRtcError::Grazer(format!(
                "Moltbook post failed ({}): {}",
                status, body
            )));
        }
        Ok(body)
    }

    async fn post_fourclaw(
        &self,
        api_key: &str,
        title: &str,
        content: &str,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let board = extra["board"].as_str().unwrap_or("b");
        let url = format!(
            "{}/api/v1/boards/{}/threads",
            Platform::FourClaw.base_url(),
            urlencoded(board)
        );
        debug!(url, board, "Posting to 4claw");
        let resp = self
            .http
            .post(&url)
            .bearer_auth(api_key)
            .json(&serde_json::json!({
                "title": title,
                "content": content,
                "anon": false,
            }))
            .send()
            .await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(ClawRtcError::Grazer(format!(
                "4claw post failed ({}): {}",
                status, body
            )));
        }
        Ok(body)
    }

    async fn post_agentchan(
        &self,
        api_key: &str,
        content: &str,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let board = extra["board"].as_str().unwrap_or("ai");
        let reply_to = extra["reply_to"].as_str();

        let url = if let Some(thread_id) = reply_to {
            format!(
                "{}/api/boards/{}/threads/{}/posts",
                Platform::Agentchan.base_url(),
                urlencoded(board),
                urlencoded(thread_id)
            )
        } else {
            format!(
                "{}/api/boards/{}/threads",
                Platform::Agentchan.base_url(),
                urlencoded(board)
            )
        };

        debug!(url, board, "Posting to AgentChan");
        let resp = self
            .http
            .post(&url)
            .bearer_auth(api_key)
            .json(&serde_json::json!({ "content": content }))
            .send()
            .await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
        if !status.is_success() {
            return Err(ClawRtcError::Grazer(format!(
                "AgentChan post failed ({}): {}",
                status, body
            )));
        }
        Ok(body)
    }

    async fn post_clawsta(
        &self,
        api_key: &str,
        content: &str,
    ) -> ClawRtcResult<serde_json::Value> {
        let url = format!("{}/v1/posts", Platform::Clawsta.base_url());
        debug!(url, "Posting to ClawSta");
        let resp = self
            .http
            .post(&url)
            .bearer_auth(api_key)
            .json(&serde_json::json!({ "content": content }))
            .send()
            .await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(ClawRtcError::Grazer(format!(
                "ClawSta post failed ({}): {}",
                status, body
            )));
        }
        Ok(body)
    }

    async fn post_clawnews(
        &self,
        api_key: &str,
        headline: &str,
        summary: &str,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let url_field = extra["url"].as_str().unwrap_or("");
        let tags: Option<Vec<&str>> = extra["tags"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect());
        let url = format!("{}/api/stories", Platform::Clawnews.base_url());
        debug!(url, "Posting to ClawNews");
        let mut body = serde_json::json!({
            "headline": headline,
            "url": url_field,
            "summary": summary,
        });
        if let Some(t) = tags {
            body["tags"] = serde_json::json!(t);
        }
        let resp = self
            .http
            .post(&url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let result: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
        if !status.is_success() {
            return Err(ClawRtcError::Grazer(format!(
                "ClawNews post failed ({}): {}",
                status, result
            )));
        }
        Ok(result)
    }

    async fn post_pinchedin(
        &self,
        api_key: &str,
        content: &str,
    ) -> ClawRtcResult<serde_json::Value> {
        let url = format!("{}/api/posts", Platform::Pinchedin.base_url());
        debug!(url, "Posting to PinchedIn");
        let resp = self
            .http
            .post(&url)
            .bearer_auth(api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "content": content }))
            .send()
            .await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(ClawRtcError::Grazer(format!(
                "PinchedIn post failed ({}): {}",
                status, body
            )));
        }
        Ok(body)
    }

    async fn post_clawtask(
        &self,
        api_key: &str,
        title: &str,
        description: &str,
        extra: &serde_json::Value,
    ) -> ClawRtcResult<serde_json::Value> {
        let deadline = extra["deadline_hours"].as_u64().unwrap_or(168);
        let tags: Option<Vec<&str>> = extra["tags"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect());
        let url = format!("{}/api/bounties", Platform::Clawtasks.base_url());
        debug!(url, "Posting to ClawTasks");
        let mut body = serde_json::json!({
            "title": title,
            "description": description,
            "deadline_hours": deadline,
        });
        if let Some(t) = tags {
            body["tags"] = serde_json::json!(t);
        }
        let resp = self
            .http
            .post(&url)
            .bearer_auth(api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let result: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(ClawRtcError::Grazer(format!(
                "ClawTasks post failed ({}): {}",
                status, result
            )));
        }
        Ok(result)
    }
}

/// Minimal percent-encoding for URL query parameters.
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
    fn test_platform_from_str() {
        assert_eq!("bottube".parse::<Platform>().unwrap(), Platform::Bottube);
        assert_eq!("4claw".parse::<Platform>().unwrap(), Platform::FourClaw);
        assert_eq!("fourclaw".parse::<Platform>().unwrap(), Platform::FourClaw);
        assert_eq!("moltbook".parse::<Platform>().unwrap(), Platform::Moltbook);
        assert!("unknown".parse::<Platform>().is_err());
    }

    #[test]
    fn test_platform_base_urls() {
        assert_eq!(Platform::Bottube.base_url(), "https://bottube.ai");
        assert_eq!(Platform::Moltbook.base_url(), "https://www.moltbook.com");
        assert_eq!(Platform::FourClaw.base_url(), "https://www.4claw.org");
    }

    #[test]
    fn test_all_platform_names() {
        assert_eq!(Platform::all_names().len(), 12);
    }

    #[test]
    fn test_urlencoded() {
        assert_eq!(urlencoded("hello world"), "hello%20world");
        assert_eq!(urlencoded("a&b=c"), "a%26b%3Dc");
    }
}
