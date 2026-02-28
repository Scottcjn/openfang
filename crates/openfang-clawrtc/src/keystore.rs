//! AES-256-GCM encrypted keystore (Python-compatible format).
//!
//! Uses Argon2id for key derivation and AES-256-GCM for encryption.
//! The JSON format matches the Python `rustchain_crypto.py` keystore.

use crate::error::{ClawRtcError, ClawRtcResult};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::Utc;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Encrypted keystore JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keystore {
    pub version: u32,
    pub address: String,
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
    pub created: String,
}

impl Keystore {
    /// Encrypt a private key hex string with a password.
    pub fn encrypt(private_key_hex: &str, password: &str, address: &str) -> ClawRtcResult<Self> {
        let mut salt = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut salt);

        let key = derive_key(password, &salt)?;

        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| ClawRtcError::KeystoreEncrypt(e.to_string()))?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, private_key_hex.as_bytes())
            .map_err(|e| ClawRtcError::KeystoreEncrypt(e.to_string()))?;

        Ok(Self {
            version: 1,
            address: address.to_string(),
            salt: B64.encode(salt),
            nonce: B64.encode(nonce_bytes),
            ciphertext: B64.encode(ciphertext),
            created: Utc::now().to_rfc3339(),
        })
    }

    /// Decrypt the keystore, returning the private key hex string.
    pub fn decrypt(&self, password: &str) -> ClawRtcResult<String> {
        let salt = B64
            .decode(&self.salt)
            .map_err(|e| ClawRtcError::KeystoreDecrypt(e.to_string()))?;
        let nonce_bytes = B64
            .decode(&self.nonce)
            .map_err(|e| ClawRtcError::KeystoreDecrypt(e.to_string()))?;
        let ciphertext = B64
            .decode(&self.ciphertext)
            .map_err(|e| ClawRtcError::KeystoreDecrypt(e.to_string()))?;

        let key = derive_key(password, &salt)?;

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| ClawRtcError::KeystoreDecrypt(e.to_string()))?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| ClawRtcError::KeystoreDecrypt("wrong password or corrupted data".into()))?;

        String::from_utf8(plaintext).map_err(|e| ClawRtcError::KeystoreDecrypt(e.to_string()))
    }

    /// Load from a JSON file.
    pub fn load(path: &Path) -> ClawRtcResult<Self> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data).map_err(|e| ClawRtcError::KeystoreDecrypt(e.to_string()))
    }

    /// Save to a JSON file with restricted permissions.
    pub fn save(&self, path: &Path) -> ClawRtcResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, &json)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
}

/// Derive a 32-byte key from password + salt using Argon2id.
fn derive_key(password: &str, salt: &[u8]) -> ClawRtcResult<[u8; 32]> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| ClawRtcError::Crypto(format!("Argon2 KDF failed: {e}")))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keystore_roundtrip() {
        let secret = "deadbeefcafebabe1234567890abcdef1234567890abcdef1234567890abcdef";
        let ks = Keystore::encrypt(secret, "strong_password_123", "RTCtest").unwrap();
        let decrypted = ks.decrypt("strong_password_123").unwrap();
        assert_eq!(decrypted, secret);
    }

    #[test]
    fn test_keystore_wrong_password() {
        let secret = "deadbeefcafebabe1234567890abcdef1234567890abcdef1234567890abcdef";
        let ks = Keystore::encrypt(secret, "correct_password", "RTCtest").unwrap();
        let result = ks.decrypt("wrong_password");
        assert!(result.is_err());
    }

    #[test]
    fn test_keystore_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_keystore.json");
        let secret = "aabbccdd11223344aabbccdd11223344aabbccdd11223344aabbccdd11223344";
        let ks = Keystore::encrypt(secret, "test_pass", "RTCtest").unwrap();
        ks.save(&path).unwrap();
        let loaded = Keystore::load(&path).unwrap();
        let decrypted = loaded.decrypt("test_pass").unwrap();
        assert_eq!(decrypted, secret);
    }
}
