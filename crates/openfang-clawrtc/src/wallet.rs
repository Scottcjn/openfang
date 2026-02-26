//! Ed25519 wallet for RustChain (RTC).
//!
//! Generates Ed25519 key pairs, derives RTC addresses, and signs transactions.
//! Address format: `"RTC"` + first 40 hex chars of `SHA-256(public_key_bytes)`.

use crate::error::{ClawRtcError, ClawRtcResult};
use crate::keystore::Keystore;
use chrono::Utc;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use zeroize::Zeroize;

/// An RTC wallet backed by an Ed25519 key pair.
pub struct RtcWallet {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    address: String,
}

/// Plaintext wallet JSON (Python-compatible format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletFile {
    pub address: String,
    pub public_key: String,
    pub private_key: String,
    pub created: String,
    pub curve: String,
    pub network: String,
}

impl RtcWallet {
    /// Generate a new random wallet.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let address = derive_address(&verifying_key);
        Self {
            signing_key,
            verifying_key,
            address,
        }
    }

    /// Restore from a hex-encoded private key (64 hex chars = 32 bytes).
    pub fn from_private_key_hex(hex_key: &str) -> ClawRtcResult<Self> {
        let bytes = hex::decode(hex_key).map_err(|e| ClawRtcError::Crypto(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(ClawRtcError::Crypto(format!(
                "Expected 32-byte private key, got {}",
                bytes.len()
            )));
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        key_bytes.zeroize();
        let verifying_key = signing_key.verifying_key();
        let address = derive_address(&verifying_key);
        Ok(Self {
            signing_key,
            verifying_key,
            address,
        })
    }

    /// Load from a plaintext wallet JSON file.
    pub fn from_file(path: &Path) -> ClawRtcResult<Self> {
        let data = std::fs::read_to_string(path)?;
        let wf: WalletFile =
            serde_json::from_str(&data).map_err(|e| ClawRtcError::Crypto(e.to_string()))?;
        Self::from_private_key_hex(&wf.private_key)
    }

    /// Load from an AES-256-GCM encrypted keystore file.
    pub fn from_keystore(path: &Path, password: &str) -> ClawRtcResult<Self> {
        let ks = Keystore::load(path)?;
        let private_key_hex = ks.decrypt(password)?;
        Self::from_private_key_hex(&private_key_hex)
    }

    /// The wallet's RTC address.
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Hex-encoded public key (64 chars).
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.verifying_key.as_bytes())
    }

    /// Hex-encoded private key (64 chars). Handle with care.
    pub fn private_key_hex(&self) -> String {
        hex::encode(self.signing_key.to_bytes())
    }

    /// Sign an arbitrary message, returning the hex-encoded signature (128 chars).
    pub fn sign(&self, message: &[u8]) -> String {
        let sig = self.signing_key.sign(message);
        hex::encode(sig.to_bytes())
    }

    /// Sign a transfer transaction, returning the full signed payload.
    pub fn sign_transaction(
        &self,
        to_address: &str,
        amount_rtc: f64,
        memo: &str,
    ) -> ClawRtcResult<serde_json::Value> {
        let nonce = Utc::now().timestamp_millis();
        let payload = serde_json::json!({
            "from": self.address,
            "to": to_address,
            "amount": amount_rtc,
            "memo": memo,
            "nonce": nonce,
        });
        let canonical = serde_json::to_string(&payload)?;
        let signature = self.sign(canonical.as_bytes());

        Ok(serde_json::json!({
            "from_address": self.address,
            "to_address": to_address,
            "amount_rtc": amount_rtc,
            "memo": memo,
            "nonce": nonce,
            "signature": signature,
            "public_key": self.public_key_hex(),
        }))
    }

    /// Save as plaintext JSON (Python-compatible format).
    pub fn save_plaintext(&self, path: &Path) -> ClawRtcResult<()> {
        let wf = WalletFile {
            address: self.address.clone(),
            public_key: self.public_key_hex(),
            private_key: self.private_key_hex(),
            created: Utc::now().to_rfc3339(),
            curve: "Ed25519".to_string(),
            network: "rustchain-mainnet".to_string(),
        };
        let json = serde_json::to_string_pretty(&wf)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &json)?;
        // Restrict permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    /// Save as an encrypted keystore file.
    pub fn save_keystore(&self, path: &Path, password: &str) -> ClawRtcResult<()> {
        let ks = Keystore::encrypt(&self.private_key_hex(), password, &self.address)?;
        ks.save(path)?;
        Ok(())
    }
}

/// Derive an RTC address from a verifying (public) key.
///
/// Format: `"RTC"` + first 40 hex chars of `SHA-256(public_key_bytes)`.
fn derive_address(verifying_key: &VerifyingKey) -> String {
    let hash = Sha256::digest(verifying_key.as_bytes());
    let hex_hash = hex::encode(hash);
    format!("RTC{}", &hex_hash[..40])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_generate() {
        let w = RtcWallet::generate();
        assert!(w.address().starts_with("RTC"));
        assert_eq!(w.address().len(), 43); // "RTC" + 40 hex
        assert_eq!(w.public_key_hex().len(), 64);
        assert_eq!(w.private_key_hex().len(), 64);
    }

    #[test]
    fn test_wallet_roundtrip_hex() {
        let w1 = RtcWallet::generate();
        let pk = w1.private_key_hex();
        let w2 = RtcWallet::from_private_key_hex(&pk).unwrap();
        assert_eq!(w1.address(), w2.address());
        assert_eq!(w1.public_key_hex(), w2.public_key_hex());
    }

    #[test]
    fn test_wallet_sign_verify() {
        let w = RtcWallet::generate();
        let sig_hex = w.sign(b"hello rustchain");
        assert_eq!(sig_hex.len(), 128); // Ed25519 signature = 64 bytes = 128 hex
    }

    #[test]
    fn test_wallet_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_wallet.json");
        let w1 = RtcWallet::generate();
        w1.save_plaintext(&path).unwrap();
        let w2 = RtcWallet::from_file(&path).unwrap();
        assert_eq!(w1.address(), w2.address());
    }

    #[test]
    fn test_address_derivation_deterministic() {
        let w = RtcWallet::generate();
        let addr1 = w.address().to_string();
        let w2 = RtcWallet::from_private_key_hex(&w.private_key_hex()).unwrap();
        assert_eq!(addr1, w2.address());
    }

    #[test]
    fn test_sign_transaction() {
        let w = RtcWallet::generate();
        let tx = w.sign_transaction("RTCdeadbeef00000000000000000000000000000000", 10.5, "test").unwrap();
        assert!(tx["signature"].as_str().unwrap().len() == 128);
        assert_eq!(tx["from_address"], w.address());
    }
}
