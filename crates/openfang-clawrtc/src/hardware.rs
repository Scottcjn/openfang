//! Hardware detection for RustChain miner classification.
//!
//! Detects CPU architecture, SIMD features, core count, memory, and MAC addresses
//! to build the attestation device payload.

use crate::error::ClawRtcResult;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Detected hardware information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    /// OS platform (e.g. "linux", "macos").
    pub platform: String,
    /// Machine architecture string (e.g. "x86_64", "ppc64", "aarch64").
    pub machine: String,
    /// Hostname.
    pub hostname: String,
    /// Device family for attestation (e.g. "x86", "arm", "powerpc").
    pub family: String,
    /// Device architecture class (e.g. "modern", "g4", "g5", "apple_silicon").
    pub arch: String,
    /// CPU model string.
    pub cpu: String,
    /// Number of logical CPU cores.
    pub cores: usize,
    /// Total memory in GB.
    pub memory_gb: u64,
    /// MAC addresses of network interfaces.
    pub macs: Vec<String>,
}

impl HardwareInfo {
    /// Detect hardware on the current system.
    pub fn detect() -> ClawRtcResult<Self> {
        let machine = std::env::consts::ARCH.to_string();
        let platform = std::env::consts::OS.to_string();
        let hostname = get_hostname();
        let cpu = get_cpu_model();
        let cores = num_cpus();
        let memory_gb = get_memory_gb();
        let macs = get_mac_addresses();
        let (family, arch) = classify_arch(&machine, &cpu);

        Ok(Self {
            platform,
            machine,
            hostname,
            family,
            arch,
            cpu,
            cores,
            memory_gb,
            macs,
        })
    }

    /// Build the `device` JSON object for attestation payloads.
    pub fn device_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "family": self.family,
            "arch": self.arch,
            "model": self.cpu,
            "cpu": self.cpu,
            "cores": self.cores,
            "memory_gb": self.memory_gb,
        })
    }

    /// Build the `signals` JSON object for attestation payloads.
    pub fn signals_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "macs": self.macs,
            "hostname": self.hostname,
        })
    }

    /// The miner ID string (e.g. "claw-myhostname").
    pub fn miner_id(&self) -> String {
        format!("claw-{}", self.hostname)
    }
}

/// Classify machine architecture into (family, arch) for RustChain multiplier lookup.
fn classify_arch(machine: &str, cpu_model: &str) -> (String, String) {
    let machine_lower = machine.to_lowercase();
    let cpu_lower = cpu_model.to_lowercase();

    // PowerPC detection
    if machine_lower.contains("ppc") || machine_lower.contains("powerpc") {
        if cpu_lower.contains("g5") || cpu_lower.contains("970") {
            return ("powerpc".into(), "g5".into());
        }
        if cpu_lower.contains("g4")
            || cpu_lower.contains("7450")
            || cpu_lower.contains("7447")
            || cpu_lower.contains("7455")
        {
            return ("powerpc".into(), "g4".into());
        }
        if cpu_lower.contains("g3") || cpu_lower.contains("750") {
            return ("powerpc".into(), "g3".into());
        }
        if cpu_lower.contains("power8") {
            return ("powerpc".into(), "power8".into());
        }
        return ("powerpc".into(), "powerpc".into());
    }

    // ARM / Apple Silicon detection
    if machine_lower.contains("arm") || machine_lower.contains("aarch64") {
        if cfg!(target_os = "macos")
            && (cpu_lower.contains("m1")
                || cpu_lower.contains("m2")
                || cpu_lower.contains("m3")
                || cpu_lower.contains("m4"))
        {
            return ("arm".into(), "apple_silicon".into());
        }
        return ("arm".into(), "modern".into());
    }

    // x86/x86_64 detection
    if cpu_lower.contains("core 2") || cpu_lower.contains("core2") {
        return ("x86".into(), "core2duo".into());
    }
    if cpu_lower.contains("pentium") {
        return ("x86".into(), "pentium4".into());
    }

    ("x86".into(), "modern".into())
}

/// Get the system hostname.
fn get_hostname() -> String {
    if let Ok(name) = std::fs::read_to_string("/etc/hostname") {
        let trimmed = name.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    if let Ok(output) = Command::new("hostname").output() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return name;
        }
    }
    "unknown".to_string()
}

/// Get CPU model string.
fn get_cpu_model() -> String {
    // Linux: parse /proc/cpuinfo
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in cpuinfo.lines() {
            let lower = line.to_lowercase();
            if lower.starts_with("model name") || lower.starts_with("cpu") {
                if let Some(val) = line.split(':').nth(1) {
                    let trimmed = val.trim().to_string();
                    if !trimmed.is_empty() {
                        return trimmed;
                    }
                }
            }
        }
    }

    // macOS: sysctl
    if let Ok(output) = Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
    {
        let model = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !model.is_empty() {
            return model;
        }
    }

    "unknown".to_string()
}

/// Get the number of logical CPUs.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Get total system memory in GB.
fn get_memory_gb() -> u64 {
    // Linux: parse /proc/meminfo
    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(kb_str) = parts.get(1) {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        return kb / 1_048_576; // KB -> GB
                    }
                }
            }
        }
    }

    // macOS: sysctl
    if let Ok(output) = Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Ok(bytes) = s.parse::<u64>() {
            return bytes / (1024 * 1024 * 1024);
        }
    }

    0
}

/// Get MAC addresses from network interfaces.
fn get_mac_addresses() -> Vec<String> {
    let mut macs = Vec::new();

    // Linux: `ip -o link`
    if let Ok(output) = Command::new("ip").args(["-o", "link"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            // Look for MAC address pattern
            if let Some(pos) = line.find("link/ether ") {
                let rest = &line[pos + 11..];
                if rest.len() >= 17 {
                    let mac = rest[..17].to_lowercase();
                    if mac != "00:00:00:00:00:00" && !macs.contains(&mac) {
                        macs.push(mac);
                    }
                }
            }
        }
    }

    // macOS fallback: `ifconfig -a`
    if macs.is_empty() {
        if let Ok(output) = Command::new("ifconfig").arg("-a").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if let Some(pos) = trimmed.find("ether ") {
                    let rest = &trimmed[pos + 6..];
                    if rest.len() >= 17 {
                        let mac = rest[..17].to_lowercase();
                        if mac != "00:00:00:00:00:00" && !macs.contains(&mac) {
                            macs.push(mac);
                        }
                    }
                }
            }
        }
    }

    if macs.is_empty() {
        macs.push("00:00:00:00:00:01".to_string());
    }
    macs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_x86_modern() {
        let (fam, arch) = classify_arch("x86_64", "AMD Ryzen 9 7950X");
        assert_eq!(fam, "x86");
        assert_eq!(arch, "modern");
    }

    #[test]
    fn test_classify_g4() {
        let (fam, arch) = classify_arch("ppc", "PowerPC G4 (7450)");
        assert_eq!(fam, "powerpc");
        assert_eq!(arch, "g4");
    }

    #[test]
    fn test_classify_g5() {
        let (fam, arch) = classify_arch("ppc64", "PowerPC G5 (970)");
        assert_eq!(fam, "powerpc");
        assert_eq!(arch, "g5");
    }

    #[test]
    fn test_classify_core2() {
        let (fam, arch) = classify_arch("x86_64", "Intel Core 2 Duo E8400");
        assert_eq!(fam, "x86");
        assert_eq!(arch, "core2duo");
    }

    #[test]
    fn test_detect_hardware() {
        let hw = HardwareInfo::detect().unwrap();
        assert!(!hw.machine.is_empty());
        assert!(hw.cores > 0);
        assert!(!hw.macs.is_empty());
    }

    #[test]
    fn test_device_payload() {
        let hw = HardwareInfo::detect().unwrap();
        let payload = hw.device_payload();
        assert!(payload["family"].is_string());
        assert!(payload["arch"].is_string());
        assert!(payload["cores"].is_number());
    }
}
