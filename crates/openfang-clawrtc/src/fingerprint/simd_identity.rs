//! Check 3: SIMD Unit Identity.
//!
//! Detects available SIMD instruction sets (SSE, AVX, AltiVec, NEON).
//! Real hardware reports actual flags; VMs may report none or generic flags.

use super::CheckResult;

pub fn check() -> CheckResult {
    let arch = std::env::consts::ARCH.to_lowercase();

    let mut flags = Vec::new();

    // Read /proc/cpuinfo flags on Linux
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in cpuinfo.lines() {
            let lower = line.to_lowercase();
            if lower.contains("flags") || lower.contains("features") {
                if let Some(val) = line.split(':').nth(1) {
                    flags = val.split_whitespace().map(|s| s.to_string()).collect();
                    break;
                }
            }
        }
    }

    // macOS fallback: sysctl for features
    if flags.is_empty() {
        if let Ok(output) = std::process::Command::new("sysctl")
            .arg("-a")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let lower = line.to_lowercase();
                if lower.contains("feature") || lower.contains("altivec") {
                    if let Some(val) = line.split(':').next_back() {
                        let trimmed = val.trim().to_string();
                        if !trimmed.is_empty() {
                            flags.push(trimmed);
                        }
                    }
                }
            }
        }
    }

    let has_sse = flags.iter().any(|f| f.to_lowercase().contains("sse"));
    let has_avx = flags.iter().any(|f| f.to_lowercase().contains("avx"));
    let has_altivec = flags.iter().any(|f| f.to_lowercase().contains("altivec"))
        || arch.contains("ppc");
    let has_neon = flags.iter().any(|f| f.to_lowercase().contains("neon"))
        || arch.contains("arm")
        || arch.contains("aarch64");

    // Also use Rust's compile-time detection for x86
    #[cfg(target_arch = "x86_64")]
    let (has_sse, has_avx) = {
        (
            has_sse || std::arch::is_x86_feature_detected!("sse2"),
            has_avx || std::arch::is_x86_feature_detected!("avx"),
        )
    };

    let sample_flags: Vec<&String> = flags.iter().take(10).collect();

    let data = serde_json::json!({
        "arch": arch,
        "simd_flags_count": flags.len(),
        "has_sse": has_sse,
        "has_avx": has_avx,
        "has_altivec": has_altivec,
        "has_neon": has_neon,
        "sample_flags": sample_flags,
    });

    // PASS if any SIMD capability detected or any flags reported
    let valid = has_sse || has_avx || has_altivec || has_neon || !flags.is_empty();

    CheckResult {
        passed: valid,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_identity_runs() {
        let result = check();
        assert!(result.data["arch"].is_string());
    }
}
