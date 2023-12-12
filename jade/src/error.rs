use std::time::SystemTimeError;

use serde::{Deserialize, Serialize};
use serde_cbor::Value;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Jade Error: {0}")]
    JadeError(ErrorDetails),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("SystemTime Error: {0}")]
    SystemTimeError(SystemTimeError),

    #[cfg(feature = "serial")]
    #[error("Serial Error: {0}")]
    SerialError(#[from] serialport::Error),

    #[error("No available ports")]
    NoAvailablePorts,

    #[error("Jade returned neither an error nor a result")]
    JadeNeitherErrorNorResult,

    #[error(transparent)]
    SerdeCbor(#[from] serde_cbor::Error),

    #[error(transparent)]
    Bip32(#[from] elements::bitcoin::bip32::Error),

    #[error("Mismatching network, jade was initialized with: {init} but the method params received {passed}")]
    MismatchingXpub {
        init: crate::Network,
        passed: crate::Network,
    },

    #[error("Poison error: {0}")]
    PoisonError(String),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorDetails {
    code: i64,
    message: String,
    data: Option<Value>,
}

impl std::fmt::Display for ErrorDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error code: {} - message: {}", self.code, self.message)
    }
}
