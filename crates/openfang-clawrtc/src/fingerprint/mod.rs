//! Hardware fingerprint checks for RIP-PoA attestation.
//!
//! Six checks validate that a miner is running on real hardware, not a VM or emulator.
//! All checks return `(passed: bool, data: serde_json::Value)`.

pub mod anti_emulation;
pub mod cache_timing;
pub mod clock_drift;
pub mod instruction_jitter;
pub mod simd_identity;
pub mod thermal_drift;

use serde::{Deserialize, Serialize};

/// Result of a single fingerprint check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub passed: bool,
    pub data: serde_json::Value,
}

/// Full fingerprint report across all 6 checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintReport {
    pub all_passed: bool,
    pub checks: FingerprintChecks,
}

/// Individual check results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintChecks {
    pub clock_drift: CheckResult,
    pub cache_timing: CheckResult,
    pub simd_identity: CheckResult,
    pub thermal_drift: CheckResult,
    pub instruction_jitter: CheckResult,
    pub anti_emulation: CheckResult,
}

/// Run all 6 fingerprint checks synchronously.
///
/// This is CPU-intensive. In async contexts, wrap in `tokio::task::spawn_blocking`.
pub fn validate_all_checks() -> FingerprintReport {
    let clock_drift = clock_drift::check();
    let cache_timing = cache_timing::check();
    let simd_identity = simd_identity::check();
    let thermal_drift = thermal_drift::check();
    let instruction_jitter = instruction_jitter::check();
    let anti_emulation = anti_emulation::check();

    let all_passed = clock_drift.passed
        && cache_timing.passed
        && simd_identity.passed
        && thermal_drift.passed
        && instruction_jitter.passed
        && anti_emulation.passed;

    FingerprintReport {
        all_passed,
        checks: FingerprintChecks {
            clock_drift,
            cache_timing,
            simd_identity,
            thermal_drift,
            instruction_jitter,
            anti_emulation,
        },
    }
}

/// Run all checks in a blocking task suitable for async contexts.
pub async fn validate_all_checks_async() -> FingerprintReport {
    tokio::task::spawn_blocking(validate_all_checks)
        .await
        .expect("Fingerprint check task panicked")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_all_checks_runs() {
        let report = validate_all_checks();
        // On real hardware, at least some checks should pass
        // We just verify it doesn't panic
        assert!(report.checks.clock_drift.data.is_object());
        assert!(report.checks.cache_timing.data.is_object());
        assert!(report.checks.simd_identity.data.is_object());
    }

    #[tokio::test]
    async fn test_validate_async() {
        let report = validate_all_checks_async().await;
        assert!(report.checks.anti_emulation.data.is_object());
    }
}
