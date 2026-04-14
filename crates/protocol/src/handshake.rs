use core::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MAGIC: u8 = 0xCB;
pub const WIRE_VERSION: u8 = 1;
pub const HANDSHAKE_MIN_SIZE: usize = 11;
pub const HANDSHAKE_ACK_MIN_SIZE: usize = 12;

/// Semver API version packed into 6 bytes (major.minor.patch, each u16 LE).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl ApiVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
    
    pub fn is_compatible_with(&self, min: &ApiVersion) -> bool {
        self.major == min.major && (self.minor > min.minor || (self.minor == min.minor && self.patch >= min.patch))
    }

    pub fn to_bytes(&self) -> [u8; 6] {
        let mut buf = [0u8; 6];
        buf[0..2].copy_from_slice(&self.major.to_le_bytes());
        buf[2..4].copy_from_slice(&self.minor.to_le_bytes());
        buf[4..6].copy_from_slice(&self.patch.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8; 6]) -> Self {
        Self {
            major: u16::from_le_bytes([data[0], data[1]]),
            minor: u16::from_le_bytes([data[2], data[3]]),
            patch: u16::from_le_bytes([data[4], data[5]]),
        }
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CodecId {
    Json = 0,
    MessagePack = 1,
}

impl CodecId {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Json),
            1 => Some(Self::MessagePack),
            _ => None,
        }
    }
}

impl fmt::Display for CodecId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json => write!(f, "JSON"),
            Self::MessagePack => write!(f, "MessagePack"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionId {
    None = 0,
    Zstd = 1,
    Lz4 = 2,
}

impl CompressionId {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Zstd),
            2 => Some(Self::Lz4),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EncryptionId {
    None = 0,
    AesGcm = 1,
}

impl EncryptionId {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::AesGcm),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Handshake {
    pub client_api_version: ApiVersion,
    pub codec_id: u8,
    pub compression_id: u8,
    pub encryption_id: u8,
    pub encryption_params: Vec<u8>,
}

impl Handshake {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HANDSHAKE_MIN_SIZE + self.encryption_params.len());
        buf.push(MAGIC);
        buf.push(WIRE_VERSION);
        buf.extend_from_slice(&self.client_api_version.to_bytes());
        buf.push(self.codec_id);
        buf.push(self.compression_id);
        buf.push(self.encryption_id);
        buf.extend_from_slice(&self.encryption_params);
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, HandshakeError> {
        if data.len() < HANDSHAKE_MIN_SIZE {
            return Err(HandshakeError::TooShort {
                expected: HANDSHAKE_MIN_SIZE,
                got: data.len(),
            });
        }

        if data[0] != MAGIC {
            return Err(HandshakeError::BadMagic(data[0]));
        }

        if data[1] != WIRE_VERSION {
            return Err(HandshakeError::UnsupportedWireVersion(data[1]));
        }

        let version_bytes: [u8; 6] = data[2..8].try_into().unwrap();
        let client_api_version = ApiVersion::from_bytes(&version_bytes);

        Ok(Self {
            client_api_version,
            codec_id: data[8],
            compression_id: data[9],
            encryption_id: data[10],
            encryption_params: data[11..].to_vec(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HandshakeStatus {
    Accepted = 0,
    Rejected = 1,
}

#[derive(Debug, Clone)]
pub struct HandshakeAck {
    pub server_api_version: ApiVersion,
    pub status: HandshakeStatus,
    pub codec_id: u8,
    pub compression_id: u8,
    pub encryption_id: u8,
    pub trailing_data: Vec<u8>, // rejection reason or encryption params
}

impl HandshakeAck {
    pub fn accepted(
        server_api_version: ApiVersion,
        codec_id: u8,
        compression_id: u8,
        encryption_id: u8,
    ) -> Self {
        Self {
            server_api_version,
            status: HandshakeStatus::Accepted,
            codec_id,
            compression_id,
            encryption_id,
            trailing_data: Vec::new(),
        }
    }

    pub fn rejected(server_api_version: ApiVersion, reason: impl Into<String>) -> Self {
        Self {
            server_api_version,
            status: HandshakeStatus::Rejected,
            codec_id: 0,
            compression_id: 0,
            encryption_id: 0,
            trailing_data: reason.into().into_bytes(),
        }
    }

    pub fn rejection_reason(&self) -> Option<&str> {
        if self.status == HandshakeStatus::Rejected {
            std::str::from_utf8(&self.trailing_data).ok()
        } else {
            None
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HANDSHAKE_ACK_MIN_SIZE + self.trailing_data.len());
        buf.push(MAGIC);
        buf.push(WIRE_VERSION);
        buf.extend_from_slice(&self.server_api_version.to_bytes());
        buf.push(self.status as u8);
        buf.push(self.codec_id);
        buf.push(self.compression_id);
        buf.push(self.encryption_id);
        buf.extend_from_slice(&self.trailing_data);
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, HandshakeError> {
        if data.len() < HANDSHAKE_ACK_MIN_SIZE {
            return Err(HandshakeError::TooShort {
                expected: HANDSHAKE_ACK_MIN_SIZE,
                got: data.len(),
            });
        }
        if data[0] != MAGIC {
            return Err(HandshakeError::BadMagic(data[0]));
        }

        if data[1] != WIRE_VERSION {
            return Err(HandshakeError::UnsupportedWireVersion(data[1]));
        }

        let version_bytes: [u8; 6] = data[2..8].try_into().unwrap();
        let server_api_version = ApiVersion::from_bytes(&version_bytes);

        let status = match data[8] {
            0 => HandshakeStatus::Accepted,
            1 => HandshakeStatus::Rejected,
            other => return Err(HandshakeError::InvalidStatus(other)),
        };

        Ok(Self {
            server_api_version,
            status,
            codec_id: data[9],
            compression_id: data[10],
            encryption_id: data[11],
            trailing_data: data[12..].to_vec(),
        })
    }
}

#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("Handshake too short: expected at least {expected} bytes, got {got}")]
    TooShort { expected: usize, got: usize },
    #[error("Bad magic byte: expected 0xCB, got 0x{0:02X}")]
    BadMagic(u8),
    #[error("Unsupported wire version: {0} (expected {WIRE_VERSION})")]
    UnsupportedWireVersion(u8),
    #[error("Invalid handshake status: {0}")]
    InvalidStatus(u8),
}