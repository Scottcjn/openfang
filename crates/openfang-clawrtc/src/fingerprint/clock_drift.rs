//! Check 1: Clock-Skew & Oscillator Drift.
//!
//! Measures timing variance of repeated SHA-256 operations.
//! Real hardware has oscillator jitter (CV ~0.01-0.15); VMs have uniform timing (CV ~0.0001).

use super::CheckResult;
use sha2::{Digest, Sha256};
use std::time::Instant;

const SAMPLES: usize = 200;
const REFERENCE_OPS: usize = 5000;

pub fn check() -> CheckResult {
    let mut intervals = Vec::with_capacity(SAMPLES);

    for i in 0..SAMPLES {
        let data = format!("drift_{i}");
        let start = Instant::now();
        for _ in 0..REFERENCE_OPS {
            // black_box prevents the compiler from optimizing away the hash
            std::hint::black_box(Sha256::digest(data.as_bytes()));
        }
        let elapsed = start.elapsed().as_nanos() as f64;
        intervals.push(elapsed);

        // Occasional yield to let OS scheduler show real jitter
        if i % 50 == 0 {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
    let variance = intervals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / intervals.len() as f64;
    let stdev = variance.sqrt();
    let cv = if mean > 0.0 { stdev / mean } else { 0.0 };

    // Compute drift between consecutive samples
    let drift_pairs: Vec<f64> = intervals
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .collect();
    let drift_mean = drift_pairs.iter().sum::<f64>() / drift_pairs.len().max(1) as f64;
    let drift_variance = drift_pairs
        .iter()
        .map(|x| (x - drift_mean).powi(2))
        .sum::<f64>()
        / drift_pairs.len().max(1) as f64;
    let drift_stdev = drift_variance.sqrt();

    let data = serde_json::json!({
        "mean_ns": mean as i64,
        "stdev_ns": stdev as i64,
        "cv": (cv * 1_000_000.0).round() / 1_000_000.0,
        "drift_stdev": drift_stdev as i64,
    });

    // FAIL if timing is too uniform (cv < 0.0001) or no drift at all
    let valid = cv >= 0.0001 && drift_stdev > 0.0;

    CheckResult {
        passed: valid,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_drift_runs() {
        let result = check();
        assert!(result.data["cv"].as_f64().is_some());
        assert!(result.data["mean_ns"].as_i64().unwrap() > 0);
    }
}
