//! Error types for the ClawRTC crate.

/// All errors that can occur in ClawRTC operations.
#[derive(Debug, thiserror::Error)]
pub enum ClawRtcError {
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),

    #[error("Keystore decryption failed: {0}")]
    KeystoreDecrypt(String),

    #[error("Keystore encryption failed: {0}")]
    KeystoreEncrypt(String),

    #[error("Invalid RTC address: {0}")]
    InvalidAddress(String),

    #[error("Node API error: {0}")]
    NodeApi(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Attestation rejected: {0}")]
    AttestationRejected(String),

    #[error("Fingerprint check failed: {0}")]
    FingerprintFailed(String),

    #[error("Hardware detection error: {0}")]
    HardwareDetection(String),

    #[error("Grazer API error: {0}")]
    Grazer(String),

    #[error("BoTTube API error: {0}")]
    BoTTube(String),

    #[error("Missing API key: {0}")]
    MissingApiKey(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<reqwest::Error> for ClawRtcError {
    fn from(e: reqwest::Error) -> Self {
        ClawRtcError::Network(e.to_string())
    }
}

impl From<ed25519_dalek::SignatureError> for ClawRtcError {
    fn from(e: ed25519_dalek::SignatureError) -> Self {
        ClawRtcError::Crypto(e.to_string())
    }
}

/// Convenience type alias.
pub type ClawRtcResult<T> = Result<T, ClawRtcError>;
