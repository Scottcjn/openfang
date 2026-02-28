//! Check 6: Anti-Emulation Behavioral Checks.
//!
//! Scans DMI tables, environment variables, CPU hypervisor flags, cloud metadata,
//! and systemd-detect-virt to identify virtual machines and cloud instances.

use super::CheckResult;
use std::process::Command;

/// Known hypervisor/cloud vendor strings in DMI tables.
const VM_STRINGS: &[&str] = &[
    "vmware",
    "virtualbox",
    "kvm",
    "qemu",
    "xen",
    "hyperv",
    "hyper-v",
    "parallels",
    "bhyve",
    "amazon",
    "amazon ec2",
    "ec2",
    "nitro",
    "google",
    "google compute engine",
    "gce",
    "microsoft corporation",
    "azure",
    "digitalocean",
    "linode",
    "akamai",
    "vultr",
    "hetzner",
    "oracle",
    "oraclecloud",
    "ovh",
    "ovhcloud",
    "alibaba",
    "alicloud",
    "bochs",
    "innotek",
    "seabios",
];

/// DMI paths to check for VM indicators.
const DMI_PATHS: &[&str] = &[
    "/sys/class/dmi/id/product_name",
    "/sys/class/dmi/id/sys_vendor",
    "/sys/class/dmi/id/board_vendor",
    "/sys/class/dmi/id/board_name",
    "/sys/class/dmi/id/bios_vendor",
    "/sys/class/dmi/id/chassis_vendor",
    "/sys/class/dmi/id/chassis_asset_tag",
    "/proc/scsi/scsi",
];

/// Environment variables that indicate containerized/cloud environments.
const VM_ENV_VARS: &[&str] = &[
    "KUBERNETES",
    "DOCKER",
    "VIRTUAL",
    "container",
    "AWS_EXECUTION_ENV",
    "ECS_CONTAINER_METADATA_URI",
    "GOOGLE_CLOUD_PROJECT",
    "AZURE_FUNCTIONS_ENVIRONMENT",
    "WEBSITE_INSTANCE_ID",
];

pub fn check() -> CheckResult {
    let mut vm_indicators = Vec::new();

    // DMI table checks
    for path in DMI_PATHS {
        if let Ok(content) = std::fs::read_to_string(path) {
            let lower = content.trim().to_lowercase();
            for vm_str in VM_STRINGS {
                if lower.contains(vm_str) {
                    vm_indicators.push(format!("{path}:{vm_str}"));
                }
            }
        }
    }

    // Environment variable checks
    for key in VM_ENV_VARS {
        if std::env::var(key).is_ok() {
            vm_indicators.push(format!("ENV:{key}"));
        }
    }

    // CPU hypervisor flag in /proc/cpuinfo
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        if cpuinfo.to_lowercase().contains("hypervisor") {
            vm_indicators.push("cpuinfo:hypervisor".to_string());
        }
    }

    // Xen hypervisor detection
    if let Ok(content) = std::fs::read_to_string("/sys/hypervisor/type") {
        let hv_type = content.trim().to_lowercase();
        if !hv_type.is_empty() {
            vm_indicators.push(format!("sys_hypervisor:{hv_type}"));
        }
    }

    // Cloud metadata endpoint (169.254.169.254) — quick timeout
    if check_cloud_metadata() {
        vm_indicators.push("cloud_metadata:detected".to_string());
    }

    // systemd-detect-virt
    if let Ok(output) = Command::new("systemd-detect-virt").output() {
        let virt_type = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
        if !virt_type.is_empty() && virt_type != "none" {
            vm_indicators.push(format!("systemd_detect_virt:{virt_type}"));
        }
    }

    let data = serde_json::json!({
        "vm_indicators": vm_indicators,
        "indicator_count": vm_indicators.len(),
        "is_likely_vm": !vm_indicators.is_empty(),
    });

    // FAIL if any VM indicator found
    let valid = vm_indicators.is_empty();

    CheckResult {
        passed: valid,
        data,
    }
}

/// Check if the cloud metadata endpoint is reachable (indicates cloud VM).
fn check_cloud_metadata() -> bool {
    use std::io::Read;
    use std::net::{TcpStream, ToSocketAddrs};

    let addr = "169.254.169.254:80";
    if let Ok(mut addrs) = addr.to_socket_addrs() {
        if let Some(addr) = addrs.next() {
            if let Ok(mut stream) =
                TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(1))
            {
                let _ = std::io::Write::write_all(
                    &mut stream,
                    b"GET / HTTP/1.0\r\nHost: 169.254.169.254\r\nMetadata: true\r\n\r\n",
                );
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(1)));
                let mut buf = [0u8; 512];
                if let Ok(n) = stream.read(&mut buf) {
                    if n > 0 {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anti_emulation_runs() {
        let result = check();
        assert!(result.data["indicator_count"].is_number());
    }
}
