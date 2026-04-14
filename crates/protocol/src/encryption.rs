use thiserror::Error;

pub trait Encryptor: Send + Sync {
    fn id(&self) -> u8;
    fn name(&self) -> &'static str;
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, EncryptionError>;
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, EncryptionError>;
}

#[derive(Debug, Error)]
#[error("Encryption error: {message}")]
pub struct EncryptionError {
    pub message: String,
}

impl EncryptionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}
