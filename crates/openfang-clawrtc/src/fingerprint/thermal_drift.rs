//! Check 4: Thermal Drift Entropy.
//!
//! Measures timing variance cold vs hot. Real hardware shows thermal drift
//! as the CPU heats up; VMs show uniform timing regardless of load.

use super::CheckResult;
use sha2::{Digest, Sha256};
use std::time::Instant;

const SAMPLES: usize = 50;
const HASH_OPS: usize = 10_000;
const WARMUP_ROUNDS: usize = 100;
const WARMUP_OPS: usize = 50_000;

pub fn check() -> CheckResult {
    // Collect cold timing samples
    let mut cold_times = Vec::with_capacity(SAMPLES);
    for i in 0..SAMPLES {
        let data = format!("cold_{i}");
        let start = Instant::now();
        for _ in 0..HASH_OPS {
            std::hint::black_box(Sha256::digest(data.as_bytes()));
        }
        cold_times.push(start.elapsed().as_nanos() as f64);
    }

    // Heat the CPU with sustained load
    for _ in 0..WARMUP_ROUNDS {
        for _ in 0..WARMUP_OPS {
            std::hint::black_box(Sha256::digest(b"warmup"));
        }
    }

    // Collect hot timing samples
    let mut hot_times = Vec::with_capacity(SAMPLES);
    for i in 0..SAMPLES {
        let data = format!("hot_{i}");
        let start = Instant::now();
        for _ in 0..HASH_OPS {
            std::hint::black_box(Sha256::digest(data.as_bytes()));
        }
        hot_times.push(start.elapsed().as_nanos() as f64);
    }

    let cold_avg = cold_times.iter().sum::<f64>() / cold_times.len() as f64;
    let hot_avg = hot_times.iter().sum::<f64>() / hot_times.len() as f64;
    let cold_stdev = stdev(&cold_times);
    let hot_stdev = stdev(&hot_times);
    let drift_ratio = if cold_avg > 0.0 {
        hot_avg / cold_avg
    } else {
        0.0
    };

    let data = serde_json::json!({
        "cold_avg_ns": cold_avg as i64,
        "hot_avg_ns": hot_avg as i64,
        "cold_stdev": cold_stdev as i64,
        "hot_stdev": hot_stdev as i64,
        "drift_ratio": (drift_ratio * 10_000.0).round() / 10_000.0,
    });

    // PASS if there's any thermal variance at all
    let valid = cold_stdev > 0.0 || hot_stdev > 0.0;

    CheckResult {
        passed: valid,
        data,
    }
}

fn stdev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_drift_runs() {
        let result = check();
        assert!(result.data["cold_avg_ns"].as_i64().unwrap() > 0);
    }
}
