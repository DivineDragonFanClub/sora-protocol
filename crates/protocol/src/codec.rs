use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;

pub trait Codec: Send + Sync {
    fn id(&self) -> u8;
    fn name(&self) -> &'static str;
    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, CodecError>;
    fn decode<T: DeserializeOwned>(&self, data: &[u8]) -> Result<T, CodecError>;
}

#[derive(Debug, Error)]
#[error("Codec error: {message}")]
pub struct CodecError {
    pub message: String,
}

impl CodecError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}
