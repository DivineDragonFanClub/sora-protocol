use sora_protocol::codec::{Codec, CodecError};
use sora_protocol::handshake::CodecId;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Clone)]
pub struct JsonCodec;

impl Codec for JsonCodec {
    fn id(&self) -> u8 {
        CodecId::Json as u8
    }

    fn name(&self) -> &'static str {
        "JSON"
    }

    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(value).map_err(|e| CodecError::new(e.to_string()))
    }

    fn decode<T: DeserializeOwned>(&self, data: &[u8]) -> Result<T, CodecError> {
        serde_json::from_slice(data).map_err(|e| CodecError::new(e.to_string()))
    }
}