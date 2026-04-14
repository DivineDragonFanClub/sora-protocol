use serde::{Deserialize, Serialize};

/// Codec-encoded frame exchanged after the handshake completes.
/// Payloads are opaque codec-encoded bytes — the handler decodes them separately.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Frame {
    Request {
        call_id: u32,
        namespace: u16,
        command: u16,
        payload: Vec<u8>,
    },
    Response {
        call_id: u32,
        payload: Vec<u8>,
    },
    Error {
        call_id: u32,
        module: u16,
        code: u16,
        detail: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<Vec<u8>>,
    },
    Notification {
        namespace: u16,
        command: u16,
        payload: Vec<u8>,
    },
    Ping,
    Pong,
}

impl Frame {
    pub fn request(call_id: u32, namespace: u16, command: u16, payload: Vec<u8>) -> Self {
        Self::Request { call_id, namespace, command, payload }
    }

    pub fn response(call_id: u32, payload: Vec<u8>) -> Self {
        Self::Response { call_id, payload }
    }

    pub fn error(call_id: u32, module: u16, code: u16, detail: String, context: Option<Vec<u8>>) -> Self {
        Self::Error { call_id, module, code, detail, context }
    }

    pub fn error_from_rpc(call_id: u32, err: &crate::error::RpcError) -> Self {
        Self::Error {
            call_id,
            module: err.id.module,
            code: err.id.code,
            detail: err.detail.clone(),
            context: err.context.clone(),
        }
    }

    pub fn notification(namespace: u16, command: u16, payload: Vec<u8>) -> Self {
        Self::Notification { namespace, command, payload }
    }
}
