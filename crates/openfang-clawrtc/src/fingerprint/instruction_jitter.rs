//! Check 5: Instruction Path Jitter.
//!
//! Times integer, floating-point, and branch operations separately.
//! Real hardware shows different jitter profiles per pipeline; VMs flatten jitter.

use super::CheckResult;
use std::time::Instant;

const SAMPLES: usize = 100;
const OPS: usize = 10_000;

fn measure_int_ops() -> f64 {
    let start = Instant::now();
    let mut x: u64 = 1;
    for i in 0..OPS as u64 {
        x = std::hint::black_box((x.wrapping_mul(7).wrapping_add(13)) % 65537);
        std::hint::black_box(i);
    }
    std::hint::black_box(x);
    start.elapsed().as_nanos() as f64
}

fn measure_fp_ops() -> f64 {
    let start = Instant::now();
    let mut x: f64 = 1.5;
    for i in 0..OPS {
        x = std::hint::black_box((x * 1.414 + 0.5) % 1000.0);
        std::hint::black_box(i);
    }
    std::hint::black_box(x);
    start.elapsed().as_nanos() as f64
}

fn measure_branch_ops() -> f64 {
    let start = Instant::now();
    let mut x: i64 = 0;
    for i in 0..OPS {
        if i % 2 == 0 {
            x += 1;
        } else {
            x -= 1;
        }
        std::hint::black_box(x);
    }
    std::hint::black_box(x);
    start.elapsed().as_nanos() as f64
}

pub fn check() -> CheckResult {
    let mut int_times = Vec::with_capacity(SAMPLES);
    let mut fp_times = Vec::with_capacity(SAMPLES);
    let mut branch_times = Vec::with_capacity(SAMPLES);

    for _ in 0..SAMPLES {
        int_times.push(measure_int_ops());
        fp_times.push(measure_fp_ops());
        branch_times.push(measure_branch_ops());
    }

    let int_avg = mean(&int_times);
    let fp_avg = mean(&fp_times);
    let branch_avg = mean(&branch_times);
    let int_stdev = stdev(&int_times);
    let fp_stdev = stdev(&fp_times);
    let branch_stdev = stdev(&branch_times);

    let data = serde_json::json!({
        "int_avg_ns": int_avg as i64,
        "fp_avg_ns": fp_avg as i64,
        "branch_avg_ns": branch_avg as i64,
        "int_stdev": int_stdev as i64,
        "fp_stdev": fp_stdev as i64,
        "branch_stdev": branch_stdev as i64,
    });

    // PASS if any jitter across instruction types
    let valid = int_stdev > 0.0 || fp_stdev > 0.0 || branch_stdev > 0.0;

    CheckResult {
        passed: valid,
        data,
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len().max(1) as f64
}

fn stdev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    let variance = values.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_jitter_runs() {
        let result = check();
        assert!(result.data["int_avg_ns"].as_i64().unwrap() > 0);
    }
}
