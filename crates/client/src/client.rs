use std::sync::atomic::{AtomicU32, Ordering};

use sora_protocol::codec::Codec;
use sora_protocol::command::CommandId;
use sora_protocol::error::RpcError;
use sora_protocol::frame::Frame;
use sora_protocol::handshake::{
    ApiVersion, CodecId, CompressionId, EncryptionId, Handshake, HandshakeAck, HandshakeStatus,
};
use sora_protocol::transport::{Transport, TransportError};

pub struct ClientConfig {
    pub api_version: ApiVersion,
    pub codec_id: u8,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            api_version: ApiVersion::new(0, 1, 0),
            codec_id: CodecId::Json as u8,
        }
    }
}

pub struct Client<T: Transport, C: Codec> {
    transport: T,
    codec: C,
    config: ClientConfig,
    next_call_id: AtomicU32,
    server_api_version: Option<ApiVersion>,
}

impl<T: Transport, C: Codec> Client<T, C> {
    pub fn new(transport: T, codec: C, config: ClientConfig) -> Self {
        Self {
            transport,
            codec,
            config,
            next_call_id: AtomicU32::new(1),
            server_api_version: None,
        }
    }

    /// Perform the handshake with the server, ust be called before anything else
    pub fn handshake(&mut self) -> Result<ApiVersion, ClientError> {
        let hs = Handshake {
            client_api_version: self.config.api_version,
            codec_id: self.config.codec_id,
            compression_id: CompressionId::None as u8,
            encryption_id: EncryptionId::None as u8,
            encryption_params: Vec::new(),
        };

        self.transport.send_bytes(&hs.to_bytes())?;

        let ack_data = self.transport.recv_bytes()?;

        let ack = HandshakeAck::from_bytes(&ack_data)
            .map_err(|e| ClientError::Handshake(format!("Invalid handshake ack: {e}")))?;

        if ack.status == HandshakeStatus::Rejected {
            let reason = ack
                .rejection_reason()
                .unwrap_or("Unknown reason")
                .to_string();

            return Err(ClientError::Rejected(reason));
        }

        self.server_api_version = Some(ack.server_api_version);

        println!(
            "Connected to server v{} (codec={}, compression={}, encryption={})",
            ack.server_api_version,
            ack.codec_id,
            ack.compression_id,
            ack.encryption_id
        );

        Ok(ack.server_api_version)
    }

    /// Send a request and wait for the response.
    ///
    /// The request is encoded and decoded with the codec
    pub fn request(
        &mut self,
        cmd_id: CommandId,
        request: &impl serde::Serialize,
    ) -> Result<Vec<u8>, ClientError> {
        let call_id = self.next_call_id.fetch_add(1, Ordering::Relaxed);

        let payload = self
            .codec
            .encode(request)
            .map_err(|e| ClientError::Codec(format!("Failed to encode request: {e}")))?;

        let frame = Frame::request(call_id, cmd_id.namespace, cmd_id.command, payload);

        let frame_bytes = self
            .codec
            .encode(&frame)
            .map_err(|e| ClientError::Codec(format!("Failed to encode frame: {e}")))?;

        self.transport.send_bytes(&frame_bytes)?;

        // Wait for the response
        loop {
            let response_data = self.transport.recv_bytes()?;

            let response_frame: Frame = self
                .codec
                .decode(&response_data)
                .map_err(|e| ClientError::Codec(format!("Failed to decode response: {e}")))?;

            match response_frame {
                Frame::Response { call_id: resp_id, payload} if resp_id == call_id => {
                    return Ok(payload);
                }
                Frame::Error {
                    call_id: resp_id,
                    module,
                    code,
                    detail,
                    ..
                } if resp_id == call_id => {
                    return Err(ClientError::Rpc(RpcError::new(
                        sora_protocol::ErrorId::new(module, code),
                        detail,
                    )));
                }
                Frame::Ping => {
                    // Respond to server pings while we wait
                    let pong = self
                        .codec
                        .encode(&Frame::Pong)
                        .map_err(|e| ClientError::Codec(e.to_string()))?;

                    self.transport.send_bytes(&pong)?;
                }
                Frame::Notification { .. } => {
                    // TODO: deal with notifications
                    println!("Received notification while waiting for response");
                }
                _ => {
                    println!("Unexpected frame while waiting for call_id {call_id}");
                }
            }
        }
    }
    
    pub fn call<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &mut self,
        cmd_id: CommandId,
        request: &Req,
    ) -> Result<Resp, ClientError> {
        let response_bytes = self.request(cmd_id, request)?;

        self.codec
            .decode(&response_bytes)
            .map_err(|e| ClientError::Codec(format!("Failed to decode response: {e}")))
    }

    /// Send a ping and wait for pong
    pub fn ping(&mut self) -> Result<(), ClientError> {
        let ping_bytes = self
            .codec
            .encode(&Frame::Ping)
            .map_err(|e| ClientError::Codec(e.to_string()))?;

        self.transport.send_bytes(&ping_bytes)?;

        let response_data = self.transport.recv_bytes()?;

        let frame: Frame = self
            .codec
            .decode(&response_data)
            .map_err(|e| ClientError::Codec(e.to_string()))?;

        match frame {
            Frame::Pong => Ok(()),
            _ => Err(ClientError::Codec("Expected Pong, got something else".into())),
        }
    }
    
    pub fn server_version(&self) -> Option<ApiVersion> {
        self.server_api_version
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),
    #[error("Handshake failed: {0}")]
    Handshake(String),
    #[error("Server rejected connection: {0}")]
    Rejected(String),
    #[error("Codec error: {0}")]
    Codec(String),
    #[error("RPC error: {0}")]
    Rpc(RpcError),
}
