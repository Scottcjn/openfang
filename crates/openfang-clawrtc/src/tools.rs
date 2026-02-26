//! OpenFang tool integration — 15 tools for agent use.
//!
//! Covers RustChain (8 tools), Grazer (2 tools), BoTTube (3 tools), and ClawHub (1 tool).
//! Each tool is registered as a `ToolDefinition` and dispatched via `execute_clawrtc_tool()`.

use crate::bottube::BoTTubeClient;
use crate::client::RustChainClient;
use crate::fingerprint;
use crate::grazer::{GrazerClient, Platform};
use crate::hardware::HardwareInfo;
use crate::wallet::RtcWallet;
use openfang_types::tool::ToolDefinition;
use sha2::Digest;
use std::path::PathBuf;

/// Default wallet directory under ~/.clawrtc/wallets/.
fn default_wallet_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".clawrtc")
        .join("wallets")
        .join("default.json")
}

/// Return all 15 ClawRTC tool definitions for the OpenFang tool registry.
pub fn clawrtc_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // ─── RustChain tools (8) ─────────────────────────────────────────
        ToolDefinition {
            name: "rustchain_balance".to_string(),
            description: "Check the RTC token balance for a wallet address on the RustChain network.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "wallet": { "type": "string", "description": "RTC wallet address (e.g. RTCabc123...). If omitted, uses the default wallet." }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "rustchain_wallet_create".to_string(),
            description: "Generate a new Ed25519 RTC wallet. Returns the address and public key. The private key is saved to disk.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "force": { "type": "boolean", "description": "Overwrite existing wallet if true. Default false." }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "rustchain_wallet_show".to_string(),
            description: "Display the current wallet address and its RTC balance.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "rustchain_attest".to_string(),
            description: "Run hardware attestation against the RustChain network. Proves this device is real hardware.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "node_url": { "type": "string", "description": "RustChain node URL. Default: https://bulbous-bouffant.metalseed.net" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "rustchain_enroll".to_string(),
            description: "Enroll in the current RustChain epoch to earn RTC mining rewards.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "node_url": { "type": "string", "description": "RustChain node URL. Default: https://bulbous-bouffant.metalseed.net" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "rustchain_network_status".to_string(),
            description: "Check RustChain network status: node health, active miners, and version.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "node_url": { "type": "string", "description": "RustChain node URL. Default: https://bulbous-bouffant.metalseed.net" }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "rustchain_fingerprint".to_string(),
            description: "Run all 6 RIP-PoA hardware fingerprint checks (clock drift, cache timing, SIMD identity, thermal drift, instruction jitter, anti-emulation).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "rustchain_transfer".to_string(),
            description: "Send a signed RTC token transfer to another wallet.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "to": { "type": "string", "description": "Recipient RTC wallet address" },
                    "amount": { "type": "number", "description": "Amount of RTC to send" },
                    "memo": { "type": "string", "description": "Optional transfer memo" }
                },
                "required": ["to", "amount"]
            }),
        },
        // ─── Grazer tools (2) ────────────────────────────────────────────
        ToolDefinition {
            name: "grazer_discover".to_string(),
            description: "Discover trending content across Elyan Labs platforms (BoTTube, Moltbook, 4claw, ClawHub, PinchedIn, AgentChan, ClawSta, ClawNews, ClawTasks, SwarmHub, Agent Directory). Returns top posts/videos/skills from each platform.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "platform": {
                        "type": "string",
                        "description": "Specific platform to discover from (bottube, moltbook, fourclaw, clawhub, pinchedin, agentchan, clawsta, clawnews, clawtasks, swarmhub, directory). If omitted, discovers from all platforms."
                    },
                    "query": {
                        "type": "string",
                        "description": "Optional search query to filter results."
                    }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "grazer_post".to_string(),
            description: "Post content to an Elyan Labs platform (Moltbook, 4claw, AgentChan, ClawSta, ClawNews, PinchedIn, or ClawTasks). Requires an API key for the target platform.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "platform": {
                        "type": "string",
                        "description": "Target platform: moltbook, fourclaw, agentchan, clawsta, clawnews, pinchedin, or clawtasks."
                    },
                    "title": { "type": "string", "description": "Post title." },
                    "content": { "type": "string", "description": "Post body content." },
                    "api_key": { "type": "string", "description": "API key for the target platform (e.g. moltbook_sk_... or clawchan_...)." },
                    "submolt": { "type": "string", "description": "(Moltbook only) Submolt name to post to." },
                    "board": { "type": "string", "description": "(4claw only) Board name to post to." },
                    "reply_to": { "type": "string", "description": "(AgentChan only) Post ID to reply to." },
                    "category": { "type": "string", "description": "(ClawNews/ClawSta only) Content category." }
                },
                "required": ["platform", "content", "api_key"]
            }),
        },
        // ─── BoTTube tools (4) ───────────────────────────────────────────
        ToolDefinition {
            name: "bottube_search".to_string(),
            description: "Search for videos on BoTTube (bottube.ai), the AI video platform.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query." },
                    "page": { "type": "integer", "description": "Page number for pagination. Default 1." }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "bottube_trending".to_string(),
            description: "Get trending videos on BoTTube.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "bottube_comment".to_string(),
            description: "Post a comment on a BoTTube video. Requires a BoTTube API key.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "video_id": { "type": "string", "description": "The video ID to comment on." },
                    "content": { "type": "string", "description": "Comment text." },
                    "api_key": { "type": "string", "description": "BoTTube API key (bottube_sk_...)." },
                    "parent_id": { "type": "string", "description": "Optional parent comment ID for replies." }
                },
                "required": ["video_id", "content", "api_key"]
            }),
        },
        ToolDefinition {
            name: "bottube_vote".to_string(),
            description: "Like or dislike a BoTTube video. Requires a BoTTube API key.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "video_id": { "type": "string", "description": "The video ID to vote on." },
                    "vote": { "type": "integer", "description": "1 = like, -1 = dislike, 0 = remove vote." },
                    "api_key": { "type": "string", "description": "BoTTube API key (bottube_sk_...)." }
                },
                "required": ["video_id", "vote", "api_key"]
            }),
        },
        // ─── ClawHub tools (1) ───────────────────────────────────────────
        ToolDefinition {
            name: "clawhub_search".to_string(),
            description: "Search the ClawHub skill registry for agent skills, packages, and tools.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query for skills or packages." }
                },
                "required": ["query"]
            }),
        },
    ]
}

/// Execute a ClawRTC tool by name. Returns `Ok(content)` or `Err(error_message)`.
pub async fn execute_clawrtc_tool(
    tool_name: &str,
    input: &serde_json::Value,
) -> Result<String, String> {
    match tool_name {
        // RustChain tools
        "rustchain_balance" => tool_balance(input).await,
        "rustchain_wallet_create" => tool_wallet_create(input),
        "rustchain_wallet_show" => tool_wallet_show(input).await,
        "rustchain_attest" => tool_attest(input).await,
        "rustchain_enroll" => tool_enroll(input).await,
        "rustchain_network_status" => tool_network_status(input).await,
        "rustchain_fingerprint" => tool_fingerprint().await,
        "rustchain_transfer" => tool_transfer(input).await,
        // Grazer tools
        "grazer_discover" => tool_grazer_discover(input).await,
        "grazer_post" => tool_grazer_post(input).await,
        // BoTTube tools
        "bottube_search" => tool_bottube_search(input).await,
        "bottube_trending" => tool_bottube_trending().await,
        "bottube_comment" => tool_bottube_comment(input).await,
        "bottube_vote" => tool_bottube_vote(input).await,
        // ClawHub tools
        "clawhub_search" => tool_clawhub_search(input).await,
        _ => Err(format!("Unknown clawrtc tool: {tool_name}")),
    }
}

/// Check if a tool name belongs to the clawrtc module.
pub fn is_clawrtc_tool(name: &str) -> bool {
    name.starts_with("rustchain_")
        || name.starts_with("grazer_")
        || name.starts_with("bottube_")
        || name.starts_with("clawhub_")
}

// ─── Tool implementations ───────────────────────────────────────────────────

fn get_client(input: &serde_json::Value) -> RustChainClient {
    let url = input["node_url"]
        .as_str()
        .unwrap_or(crate::client::DEFAULT_NODE_URL);
    RustChainClient::new(url)
}

async fn tool_balance(input: &serde_json::Value) -> Result<String, String> {
    let wallet_addr = if let Some(addr) = input["wallet"].as_str() {
        addr.to_string()
    } else {
        let path = default_wallet_path();
        let w = RtcWallet::from_file(&path).map_err(|e| format!("No wallet found: {e}"))?;
        w.address().to_string()
    };

    let client = get_client(input);
    let balance = client
        .balance(&wallet_addr)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "wallet": wallet_addr,
        "balance_rtc": balance,
    }))
    .unwrap())
}

fn tool_wallet_create(input: &serde_json::Value) -> Result<String, String> {
    let path = default_wallet_path();
    let force = input["force"].as_bool().unwrap_or(false);

    if path.exists() && !force {
        return Err(format!(
            "Wallet already exists at {}. Use force=true to overwrite.",
            path.display()
        ));
    }

    let wallet = RtcWallet::generate();
    wallet
        .save_plaintext(&path)
        .map_err(|e| format!("Failed to save wallet: {e}"))?;

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "address": wallet.address(),
        "public_key": wallet.public_key_hex(),
        "saved_to": path.display().to_string(),
        "network": "rustchain-mainnet",
    }))
    .unwrap())
}

async fn tool_wallet_show(input: &serde_json::Value) -> Result<String, String> {
    let path = default_wallet_path();
    let wallet = RtcWallet::from_file(&path)
        .map_err(|e| format!("No wallet found at {}: {e}", path.display()))?;

    let client = get_client(input);
    let balance = client.balance(wallet.address()).await.unwrap_or(0.0);

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "address": wallet.address(),
        "public_key": wallet.public_key_hex(),
        "balance_rtc": balance,
        "wallet_file": path.display().to_string(),
    }))
    .unwrap())
}

async fn tool_attest(input: &serde_json::Value) -> Result<String, String> {
    let path = default_wallet_path();
    let wallet = RtcWallet::from_file(&path)
        .map_err(|e| format!("No wallet found: {e}"))?;

    let hw = HardwareInfo::detect().map_err(|e| e.to_string())?;
    let client = get_client(input);

    // Challenge
    let challenge = client.challenge().await.map_err(|e| e.to_string())?;
    let nonce = &challenge.nonce;

    // Entropy (blocking)
    let entropy = tokio::task::spawn_blocking(|| {
        let cycles = 48;
        let inner_loop = 25_000u64;
        let mut samples = Vec::with_capacity(cycles);
        for _ in 0..cycles {
            let start = std::time::Instant::now();
            let mut acc: u64 = 0;
            for j in 0..inner_loop {
                acc ^= std::hint::black_box((j.wrapping_mul(31)) & 0xFFFFFFFF);
            }
            std::hint::black_box(acc);
            samples.push(start.elapsed().as_nanos() as f64);
        }
        let n = samples.len() as f64;
        let mean = samples.iter().sum::<f64>() / n;
        let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        serde_json::json!({
            "mean_ns": mean,
            "variance_ns": variance,
            "sample_count": samples.len(),
        })
    })
    .await
    .unwrap();

    // Commitment
    let entropy_json = serde_json::to_string(&entropy).unwrap();
    let commitment_input = format!("{}{}{}", nonce, wallet.address(), entropy_json);
    let commitment = hex::encode(sha2::Sha256::digest(commitment_input.as_bytes()));

    let payload = serde_json::json!({
        "miner": wallet.address(),
        "miner_id": hw.miner_id(),
        "nonce": nonce,
        "report": {
            "nonce": nonce,
            "commitment": commitment,
            "derived": entropy,
            "entropy_score": entropy["variance_ns"],
        },
        "device": hw.device_payload(),
        "signals": hw.signals_payload(),
    });

    client
        .submit_attestation(&payload)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "status": "accepted",
        "miner_id": hw.miner_id(),
        "wallet": wallet.address(),
        "device_arch": hw.arch,
    }))
    .unwrap())
}

async fn tool_enroll(input: &serde_json::Value) -> Result<String, String> {
    let path = default_wallet_path();
    let wallet = RtcWallet::from_file(&path)
        .map_err(|e| format!("No wallet found: {e}"))?;

    let hw = HardwareInfo::detect().map_err(|e| e.to_string())?;
    let client = get_client(input);

    let payload = serde_json::json!({
        "miner_pubkey": wallet.address(),
        "miner_id": hw.miner_id(),
        "device": {
            "family": hw.family,
            "arch": hw.arch,
        },
    });

    let resp = client.enroll(&payload).await.map_err(|e| e.to_string())?;

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "enrolled": resp.ok,
        "epoch": resp.epoch,
        "weight": resp.weight,
    }))
    .unwrap())
}

async fn tool_network_status(input: &serde_json::Value) -> Result<String, String> {
    let client = get_client(input);

    let health = client.health().await.map_err(|e| e.to_string())?;
    let miners = client.miners().await.unwrap_or_default();

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "node": client.base_url(),
        "healthy": health.ok,
        "version": health.version,
        "uptime_s": health.uptime_s,
        "active_miners": miners.len(),
        "miners": miners,
    }))
    .unwrap())
}

async fn tool_fingerprint() -> Result<String, String> {
    let report = fingerprint::validate_all_checks_async().await;

    let mut summary = Vec::new();
    let checks = &report.checks;
    summary.push(format!("Clock Drift:        {}", pass_fail(checks.clock_drift.passed)));
    summary.push(format!("Cache Timing:       {}", pass_fail(checks.cache_timing.passed)));
    summary.push(format!("SIMD Identity:      {}", pass_fail(checks.simd_identity.passed)));
    summary.push(format!("Thermal Drift:      {}", pass_fail(checks.thermal_drift.passed)));
    summary.push(format!("Instruction Jitter: {}", pass_fail(checks.instruction_jitter.passed)));
    summary.push(format!("Anti-Emulation:     {}", pass_fail(checks.anti_emulation.passed)));

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "all_passed": report.all_passed,
        "summary": summary,
        "checks": report.checks,
    }))
    .unwrap())
}

async fn tool_transfer(input: &serde_json::Value) -> Result<String, String> {
    let to = input["to"]
        .as_str()
        .ok_or("Missing required field: to")?;
    let amount = input["amount"]
        .as_f64()
        .ok_or("Missing required field: amount")?;
    let memo = input["memo"].as_str().unwrap_or("");

    if !to.starts_with("RTC") || to.len() != 43 {
        return Err(format!("Invalid RTC address: {to}"));
    }
    if amount <= 0.0 {
        return Err("Amount must be positive".to_string());
    }

    let path = default_wallet_path();
    let wallet = RtcWallet::from_file(&path)
        .map_err(|e| format!("No wallet found: {e}"))?;

    let tx_payload = wallet
        .sign_transaction(to, amount, memo)
        .map_err(|e| e.to_string())?;

    let client = get_client(input);
    let result = client
        .transfer_signed(&tx_payload)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_string_pretty(&result).unwrap())
}

// ─── Grazer tool implementations ─────────────────────────────────────────────

async fn tool_grazer_discover(input: &serde_json::Value) -> Result<String, String> {
    let grazer = GrazerClient::new();
    let extra = input.clone();

    if let Some(platform_str) = input["platform"].as_str() {
        let platform: Platform = platform_str
            .parse()
            .map_err(|e: String| e)?;

        let result = grazer
            .discover(platform, None, 20, &extra)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "platform": platform_str,
            "results": result,
        }))
        .unwrap())
    } else {
        // Discover from all platforms (best-effort, skip auth-required ones)
        let platforms = Platform::all_names();
        let mut results = serde_json::Map::new();

        for name in platforms {
            if let Ok(platform) = name.parse::<Platform>() {
                match grazer.discover(platform, None, 10, &extra).await {
                    Ok(data) => {
                        results.insert(name.to_string(), data);
                    }
                    Err(_) => {
                        results.insert(
                            name.to_string(),
                            serde_json::json!({"error": "unavailable"}),
                        );
                    }
                }
            }
        }

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "platforms_queried": platforms.len(),
            "results": results,
        }))
        .unwrap())
    }
}

async fn tool_grazer_post(input: &serde_json::Value) -> Result<String, String> {
    let platform_str = input["platform"]
        .as_str()
        .ok_or("Missing required field: platform")?;
    let content = input["content"]
        .as_str()
        .ok_or("Missing required field: content")?;
    let api_key = input["api_key"]
        .as_str()
        .ok_or("Missing required field: api_key")?;

    let platform: Platform = platform_str
        .parse()
        .map_err(|e: String| e)?;
    let grazer = GrazerClient::new();

    let title = input["title"].as_str().unwrap_or("");

    // Build extra context for platform-specific fields
    let extra = input.clone();

    let result = grazer
        .post(platform, api_key, title, content, &extra)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "platform": platform_str,
        "posted": true,
        "result": result,
    }))
    .unwrap())
}

// ─── BoTTube tool implementations ────────────────────────────────────────────

async fn tool_bottube_search(input: &serde_json::Value) -> Result<String, String> {
    let query = input["query"]
        .as_str()
        .ok_or("Missing required field: query")?;
    let page = input["page"].as_u64().unwrap_or(1) as u32;

    let client = BoTTubeClient::new(None);
    let results = client
        .search(query, page)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "query": query,
        "page": page,
        "results": results,
    }))
    .unwrap())
}

async fn tool_bottube_trending() -> Result<String, String> {
    let client = BoTTubeClient::new(None);
    let results = client.trending().await.map_err(|e| e.to_string())?;

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "trending": results,
    }))
    .unwrap())
}

async fn tool_bottube_comment(input: &serde_json::Value) -> Result<String, String> {
    let video_id = input["video_id"]
        .as_str()
        .ok_or("Missing required field: video_id")?;
    let content = input["content"]
        .as_str()
        .ok_or("Missing required field: content")?;
    let api_key = input["api_key"]
        .as_str()
        .ok_or("Missing required field: api_key")?;
    let parent_id = input["parent_id"].as_str();

    let client = BoTTubeClient::new(Some(api_key));
    let result = client
        .comment(video_id, content, parent_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "commented": true,
        "video_id": video_id,
        "result": result,
    }))
    .unwrap())
}

async fn tool_bottube_vote(input: &serde_json::Value) -> Result<String, String> {
    let video_id = input["video_id"]
        .as_str()
        .ok_or("Missing required field: video_id")?;
    let vote = input["vote"]
        .as_i64()
        .ok_or("Missing required field: vote")? as i8;
    let api_key = input["api_key"]
        .as_str()
        .ok_or("Missing required field: api_key")?;

    let client = BoTTubeClient::new(Some(api_key));
    let result = client
        .vote(video_id, vote)
        .await
        .map_err(|e| e.to_string())?;

    let action = match vote {
        1 => "liked",
        -1 => "disliked",
        _ => "unvoted",
    };

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "action": action,
        "video_id": video_id,
        "result": result,
    }))
    .unwrap())
}

// ─── ClawHub tool implementations ────────────────────────────────────────────

async fn tool_clawhub_search(input: &serde_json::Value) -> Result<String, String> {
    let query = input["query"]
        .as_str()
        .ok_or("Missing required field: query")?;

    let grazer = GrazerClient::new();
    let results = grazer
        .search_clawhub(query, 20)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "query": query,
        "results": results,
    }))
    .unwrap())
}

fn pass_fail(passed: bool) -> &'static str {
    if passed { "PASS" } else { "FAIL" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions_count() {
        let defs = clawrtc_tool_definitions();
        assert_eq!(defs.len(), 15);
    }

    #[test]
    fn test_tool_definitions_names() {
        let defs = clawrtc_tool_definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        // RustChain tools
        assert!(names.contains(&"rustchain_balance"));
        assert!(names.contains(&"rustchain_wallet_create"));
        assert!(names.contains(&"rustchain_wallet_show"));
        assert!(names.contains(&"rustchain_attest"));
        assert!(names.contains(&"rustchain_enroll"));
        assert!(names.contains(&"rustchain_network_status"));
        assert!(names.contains(&"rustchain_fingerprint"));
        assert!(names.contains(&"rustchain_transfer"));
        // Grazer tools
        assert!(names.contains(&"grazer_discover"));
        assert!(names.contains(&"grazer_post"));
        // BoTTube tools
        assert!(names.contains(&"bottube_search"));
        assert!(names.contains(&"bottube_trending"));
        assert!(names.contains(&"bottube_comment"));
        assert!(names.contains(&"bottube_vote"));
        // ClawHub tools
        assert!(names.contains(&"clawhub_search"));
    }

    #[test]
    fn test_tool_definitions_have_schemas() {
        for def in clawrtc_tool_definitions() {
            assert!(def.input_schema.is_object(), "Tool {} missing schema", def.name);
            assert!(
                def.input_schema["type"].as_str() == Some("object"),
                "Tool {} schema not object type",
                def.name
            );
        }
    }

    #[test]
    fn test_is_clawrtc_tool() {
        assert!(is_clawrtc_tool("rustchain_balance"));
        assert!(is_clawrtc_tool("rustchain_transfer"));
        assert!(is_clawrtc_tool("grazer_discover"));
        assert!(is_clawrtc_tool("grazer_post"));
        assert!(is_clawrtc_tool("bottube_search"));
        assert!(is_clawrtc_tool("bottube_trending"));
        assert!(is_clawrtc_tool("bottube_comment"));
        assert!(is_clawrtc_tool("bottube_vote"));
        assert!(is_clawrtc_tool("clawhub_search"));
        assert!(!is_clawrtc_tool("file_read"));
        assert!(!is_clawrtc_tool("web_search"));
    }
}
