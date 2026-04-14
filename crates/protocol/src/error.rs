use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Compact error identifier inspired by nn::Result — module says who failed, code says what.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ErrorId {
    pub module: u16,
    pub code: u16,
}

impl ErrorId {
    pub const fn new(module: u16, code: u16) -> Self {
        Self { module, code }
    }

    pub const fn core(code: u16) -> Self {
        Self { module: 0, code }
    }

    pub const fn application(module: u16, code: u16) -> Self {
        Self { module, code }
    }
}

impl std::fmt::Display for ErrorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{:04}", self.module, self.code)
    }
}

// Module 0 = protocol core. 1-255 = reserved. 256+ = application/plugin.
pub mod core_errors {
    use super::ErrorId;

    pub const UNKNOWN: ErrorId = ErrorId::core(1);
    pub const INVALID_FRAME: ErrorId = ErrorId::core(2);
    pub const HANDSHAKE_REQUIRED: ErrorId = ErrorId::core(3);
    pub const UNSUPPORTED_CODEC: ErrorId = ErrorId::core(4);
    pub const UNSUPPORTED_COMPRESSION: ErrorId = ErrorId::core(5);
    pub const UNSUPPORTED_ENCRYPTION: ErrorId = ErrorId::core(6);
    pub const VERSION_MISMATCH: ErrorId = ErrorId::core(7);
    pub const MALFORMED_PAYLOAD: ErrorId = ErrorId::core(8);

    pub const COMMAND_NOT_FOUND: ErrorId = ErrorId::core(100);
    pub const INVALID_PARAMS: ErrorId = ErrorId::core(101);
    pub const COMMAND_FAILED: ErrorId = ErrorId::core(102);
    pub const NOT_IMPLEMENTED: ErrorId = ErrorId::core(103);

    pub const NOT_READY: ErrorId = ErrorId::core(200);
    pub const BUSY: ErrorId = ErrorId::core(201);
    pub const UNAUTHORIZED: ErrorId = ErrorId::core(202);
}

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[error("[{id}] {detail}")]
pub struct RpcError {
    pub id: ErrorId,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<u8>>,
}

impl RpcError {
    pub fn new(id: ErrorId, detail: impl Into<String>) -> Self {
        Self {
            id,
            detail: detail.into(),
            context: None,
        }
    }

    pub fn with_context(mut self, context: Vec<u8>) -> Self {
        self.context = Some(context);
        self
    }

    pub fn unknown(detail: impl Into<String>) -> Self {
        Self::new(core_errors::UNKNOWN, detail)
    }

    pub fn invalid_frame(detail: impl Into<String>) -> Self {
        Self::new(core_errors::INVALID_FRAME, detail)
    }

    pub fn handshake_required() -> Self {
        Self::new(
            core_errors::HANDSHAKE_REQUIRED,
            "Handshake must be completed before sending commands",
        )
    }

    pub fn command_not_found(namespace: u16, command: u16) -> Self {
        Self::new(
            core_errors::COMMAND_NOT_FOUND,
            format!("No handler registered for command ({namespace}, {command})"),
        )
    }

    pub fn invalid_params(detail: impl Into<String>) -> Self {
        Self::new(core_errors::INVALID_PARAMS, detail)
    }

    pub fn command_failed(detail: impl Into<String>) -> Self {
        Self::new(core_errors::COMMAND_FAILED, detail)
    }

    pub fn malformed_payload(detail: impl Into<String>) -> Self {
        Self::new(core_errors::MALFORMED_PAYLOAD, detail)
    }
}
