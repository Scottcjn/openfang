//! Mining loop: attestation, enrollment, and reward cycle.
//!
//! Matches the Python miner protocol exactly for wire compatibility.

use crate::client::{RustChainClient, BLOCK_TIME};
use crate::error::ClawRtcResult;
use crate::fingerprint;
use crate::hardware::HardwareInfo;
use crate::wallet::RtcWallet;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Mining configuration.
pub struct MinerConfig {
    pub node_url: String,
    pub wallet: RtcWallet,
    pub run_fingerprints: bool,
}

/// RustChain miner — handles attestation, enrollment, and mining cycles.
pub struct Miner {
    client: RustChainClient,
    wallet: RtcWallet,
    hardware: HardwareInfo,
    miner_id: String,
    run_fingerprints: bool,
    attestation_valid_until: Instant,
}

impl Miner {
    /// Create a new miner instance.
    pub fn new(config: MinerConfig) -> ClawRtcResult<Self> {
        let hardware = HardwareInfo::detect()?;
        let miner_id = hardware.miner_id();
        let client = RustChainClient::new(&config.node_url);

        Ok(Self {
            client,
            wallet: config.wallet,
            hardware,
            miner_id,
            run_fingerprints: config.run_fingerprints,
            attestation_valid_until: Instant::now(), // expired — will attest on first cycle
        })
    }

    /// Run a single attestation (challenge → collect entropy → submit).
    pub async fn attest(&mut self) -> ClawRtcResult<()> {
        info!(miner_id = %self.miner_id, "Starting attestation");

        // 1. Get challenge nonce
        let challenge = self.client.challenge().await?;
        let nonce = &challenge.nonce;
        debug!(nonce, "Got attestation challenge");

        // 2. Collect timing entropy (CPU-bound, run in blocking task)
        let entropy = tokio::task::spawn_blocking(collect_entropy)
            .await
            .expect("Entropy collection panicked");

        // 3. Compute commitment hash
        let entropy_json = serde_json::to_string(&entropy)?;
        let commitment_input = format!("{}{}{}", nonce, self.wallet.address(), entropy_json);
        let commitment = hex::encode(Sha256::digest(commitment_input.as_bytes()));

        // 4. Run fingerprint checks if enabled
        let fingerprint_payload = if self.run_fingerprints {
            let report = fingerprint::validate_all_checks_async().await;
            Some(serde_json::json!({
                "all_passed": report.all_passed,
                "checks": report.checks,
            }))
        } else {
            None
        };

        // 5. Build attestation payload (matches Python format)
        let mut payload = serde_json::json!({
            "miner": self.wallet.address(),
            "miner_id": self.miner_id,
            "nonce": nonce,
            "report": {
                "nonce": nonce,
                "commitment": commitment,
                "derived": entropy,
                "entropy_score": entropy["variance_ns"],
            },
            "device": self.hardware.device_payload(),
            "signals": self.hardware.signals_payload(),
        });

        if let Some(fp) = fingerprint_payload {
            payload["fingerprint"] = fp;
        }

        // 6. Submit
        self.client.submit_attestation(&payload).await?;
        // Attestation valid for 24 hours
        self.attestation_valid_until = Instant::now() + Duration::from_secs(86400);
        info!(miner_id = %self.miner_id, "Attestation accepted");
        Ok(())
    }

    /// Enroll in the current epoch.
    pub async fn enroll(&self) -> ClawRtcResult<bool> {
        let payload = serde_json::json!({
            "miner_pubkey": self.wallet.address(),
            "miner_id": self.miner_id,
            "device": {
                "family": self.hardware.family,
                "arch": self.hardware.arch,
            },
        });

        match self.client.enroll(&payload).await {
            Ok(resp) => {
                if resp.ok {
                    info!(
                        epoch = resp.epoch,
                        weight = resp.weight,
                        "Enrolled in epoch"
                    );
                    Ok(true)
                } else {
                    warn!(error = ?resp.error, "Enrollment rejected");
                    Ok(false)
                }
            }
            Err(e) => {
                warn!(error = %e, "Enrollment failed");
                Ok(false)
            }
        }
    }

    /// Check current balance.
    pub async fn check_balance(&self) -> ClawRtcResult<f64> {
        self.client.balance(self.wallet.address()).await
    }

    /// Run the mining loop until cancelled.
    pub async fn mine_loop(&mut self, cancel: Arc<AtomicBool>) -> ClawRtcResult<()> {
        let mut cycle = 0u64;

        loop {
            if cancel.load(Ordering::Relaxed) {
                info!("Mining loop cancelled");
                break;
            }

            cycle += 1;
            info!(cycle, miner_id = %self.miner_id, "Mining cycle");

            // Re-attest if needed
            if Instant::now() >= self.attestation_valid_until {
                if let Err(e) = self.attest().await {
                    error!(error = %e, "Attestation failed");
                    if interruptible_sleep(Duration::from_secs(60), &cancel).await {
                        break;
                    }
                    continue;
                }
            }

            // Enroll
            if self.enroll().await? {
                // Wait for block time
                info!("Enrolled — waiting {} seconds for epoch", BLOCK_TIME);
                if interruptible_sleep(Duration::from_secs(BLOCK_TIME), &cancel).await {
                    break;
                }

                // Check balance after epoch
                match self.check_balance().await {
                    Ok(bal) => info!(balance = bal, "Current RTC balance"),
                    Err(e) => warn!(error = %e, "Balance check failed"),
                }
            } else {
                // Retry after 60s
                if interruptible_sleep(Duration::from_secs(60), &cancel).await {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Get the miner ID.
    pub fn miner_id(&self) -> &str {
        &self.miner_id
    }

    /// Get the wallet address.
    pub fn wallet_address(&self) -> &str {
        self.wallet.address()
    }
}

/// Sleep for a duration, checking the cancel flag every second.
/// Returns `true` if cancelled, `false` if sleep completed normally.
async fn interruptible_sleep(duration: Duration, cancel: &AtomicBool) -> bool {
    let start = Instant::now();
    while start.elapsed() < duration {
        if cancel.load(Ordering::Relaxed) {
            return true;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    false
}

/// Collect CPU timing entropy (must run on a blocking thread).
fn collect_entropy() -> serde_json::Value {
    let cycles = 48;
    let inner_loop = 25_000u64;
    let mut samples = Vec::with_capacity(cycles);

    for _ in 0..cycles {
        let start = Instant::now();
        let mut acc: u64 = 0;
        for j in 0..inner_loop {
            acc ^= std::hint::black_box((j.wrapping_mul(31)) & 0xFFFFFFFF);
        }
        std::hint::black_box(acc);
        let duration = start.elapsed().as_nanos() as f64;
        samples.push(duration);
    }

    let n = samples.len() as f64;
    let mean_ns = samples.iter().sum::<f64>() / n;
    let variance_ns = samples.iter().map(|x| (x - mean_ns).powi(2)).sum::<f64>() / n;
    let min_ns = samples.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_ns = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let preview: Vec<f64> = samples.iter().take(12).copied().collect();

    serde_json::json!({
        "mean_ns": mean_ns,
        "variance_ns": variance_ns,
        "min_ns": min_ns,
        "max_ns": max_ns,
        "sample_count": samples.len(),
        "samples_preview": preview,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_entropy() {
        let entropy = collect_entropy();
        assert!(entropy["mean_ns"].as_f64().unwrap() > 0.0);
        assert!(entropy["sample_count"].as_u64().unwrap() == 48);
    }
}
