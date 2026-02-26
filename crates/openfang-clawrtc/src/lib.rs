//! `openfang-clawrtc` — RustChain (RTC) integration for OpenFang Agent OS.
//!
//! Provides:
//! - **Wallet**: Ed25519 key pair generation, signing, encrypted keystore
//! - **Mining**: Hardware attestation, epoch enrollment, reward cycles
//! - **Fingerprints**: 6 RIP-PoA hardware validation checks
//! - **Tools**: 15 OpenFang tool definitions for agent use
//! - **Client**: Async HTTP client for RustChain node API
//! - **Grazer**: Multi-platform content discovery and posting (12 platforms)
//! - **BoTTube**: Video platform search, commenting, and voting

pub mod bottube;
pub mod client;
pub mod error;
pub mod fingerprint;
pub mod grazer;
pub mod hardware;
pub mod keystore;
pub mod miner;
pub mod tools;
pub mod wallet;

// Re-exports for convenience
pub use bottube::BoTTubeClient;
pub use client::{RustChainClient, DEFAULT_NODE_URL};
pub use error::{ClawRtcError, ClawRtcResult};
pub use fingerprint::FingerprintReport;
pub use grazer::{GrazerClient, Platform};
pub use hardware::HardwareInfo;
pub use keystore::Keystore;
pub use tools::{clawrtc_tool_definitions, execute_clawrtc_tool, is_clawrtc_tool};
pub use wallet::RtcWallet;
