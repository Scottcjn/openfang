//! Check 2: Cache Timing Fingerprint.
//!
//! Measures memory access latency at L1, L2, and L3 cache sizes.
//! Real hardware shows a clear hierarchy (L2 slower than L1, L3 slower than L2).
//! VMs often show flat timing with no hierarchy.

use super::CheckResult;
use std::time::Instant;

const ITERATIONS: usize = 100;
const ACCESSES: usize = 1000;

fn measure_access_time(buffer_size: usize) -> f64 {
    let mut buf = vec![0u8; buffer_size];
    // Touch the buffer to ensure it's allocated
    for i in (0..buffer_size).step_by(64) {
        buf[i] = (i % 256) as u8;
    }

    let start = Instant::now();
    for i in 0..ACCESSES {
        let idx = (i * 64) % buffer_size;
        // black_box prevents the compiler from optimizing away the read
        std::hint::black_box(buf[idx]);
    }
    let elapsed = start.elapsed().as_nanos() as f64;
    elapsed / ACCESSES as f64
}

pub fn check() -> CheckResult {
    let l1_size = 8 * 1024; // 8 KB
    let l2_size = 128 * 1024; // 128 KB
    let l3_size = 4 * 1024 * 1024; // 4 MB

    let mut l1_times = Vec::with_capacity(ITERATIONS);
    let mut l2_times = Vec::with_capacity(ITERATIONS);
    let mut l3_times = Vec::with_capacity(ITERATIONS);

    for _ in 0..ITERATIONS {
        l1_times.push(measure_access_time(l1_size));
        l2_times.push(measure_access_time(l2_size));
        l3_times.push(measure_access_time(l3_size));
    }

    let l1_avg = l1_times.iter().sum::<f64>() / l1_times.len() as f64;
    let l2_avg = l2_times.iter().sum::<f64>() / l2_times.len() as f64;
    let l3_avg = l3_times.iter().sum::<f64>() / l3_times.len() as f64;

    let l2_l1_ratio = if l1_avg > 0.0 { l2_avg / l1_avg } else { 0.0 };
    let l3_l2_ratio = if l2_avg > 0.0 { l3_avg / l2_avg } else { 0.0 };

    let data = serde_json::json!({
        "l1_ns": (l1_avg * 100.0).round() / 100.0,
        "l2_ns": (l2_avg * 100.0).round() / 100.0,
        "l3_ns": (l3_avg * 100.0).round() / 100.0,
        "l2_l1_ratio": (l2_l1_ratio * 1000.0).round() / 1000.0,
        "l3_l2_ratio": (l3_l2_ratio * 1000.0).round() / 1000.0,
    });

    // PASS if we see at least some cache hierarchy (ratio > 1.01) and non-zero latencies
    let valid = (l2_l1_ratio >= 1.01 || l3_l2_ratio >= 1.01) && l1_avg > 0.0 && l2_avg > 0.0 && l3_avg > 0.0;

    CheckResult {
        passed: valid,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_timing_runs() {
        let result = check();
        assert!(result.data["l1_ns"].as_f64().is_some());
    }
}
