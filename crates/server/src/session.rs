use sora_protocol::codec::Codec;
use sora_protocol::command::CommandId;
use sora_protocol::frame::Frame;
use sora_protocol::handshake::{CompressionId, EncryptionId, Handshake, HandshakeAck};
use sora_protocol::registry::CommandRegistry;
use sora_protocol::transport::{Transport, TransportError};
use sora_protocol::ApiVersion;

use crate::dispatch::Dispatcher;

enum SessionState {
    AwaitingHandshake,
    Active,
}

pub struct SessionConfig {
    pub server_api_version: ApiVersion,
    pub min_client_version: ApiVersion,
    pub supported_codecs: Vec<u8>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            server_api_version: ApiVersion::new(0, 1, 0),
            min_client_version: ApiVersion::new(0, 1, 0),
            supported_codecs: vec![sora_protocol::handshake::CodecId::Json as u8],
        }
    }
}
pub struct Session<T: Transport, C: Codec> {
    transport: T,
    codec: C,
    state: SessionState,
    config: SessionConfig,
}

impl<T: Transport, C: Codec> Session<T, C> {
    pub fn new(transport: T, codec: C, config: SessionConfig) -> Self {
        Self {
            transport,
            codec,
            state: SessionState::AwaitingHandshake,
            config,
        }
    }
    
    pub fn run(&mut self, registry: &CommandRegistry) -> Result<(), SessionError> {
        self.handle_handshake()?;

        loop {
            match self.recv_and_dispatch(registry) {
                Ok(()) => {}
                Err(SessionError::Transport(TransportError::Closed)) => {
                    println!("Client disconnected");
                    return Ok(());
                }
                Err(SessionError::Transport(TransportError::TimedOut)) => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn handle_handshake(&mut self) -> Result<(), SessionError> {
        let data = self.transport.recv_bytes()?;

        let handshake = Handshake::from_bytes(&data).map_err(|e| {
            println!("Invalid handshake: {e}");
            SessionError::InvalidHandshake(e.to_string())
        })?;

        println!(
            "Handshake received: version={}, codec={}, compression={}, encryption={}",
            handshake.client_api_version,
            handshake.codec_id,
            handshake.compression_id,
            handshake.encryption_id
        );

        // Make sure the API versions are compatible
        if !handshake.client_api_version.is_compatible_with(&self.config.min_client_version) {
            let reason = format!("Client version {} is not compatible with minimum required {}", handshake.client_api_version, self.config.min_client_version);
            let ack = HandshakeAck::rejected(self.config.server_api_version, &reason);
            self.transport.send_bytes(&ack.to_bytes())?;
            return Err(SessionError::Rejected(reason));
        }

        if !self.config.supported_codecs.contains(&handshake.codec_id) {
            let reason = format!("Unsupported codec: {}", handshake.codec_id);
            let ack = HandshakeAck::rejected(self.config.server_api_version, &reason);
            self.transport.send_bytes(&ack.to_bytes())?;
            return Err(SessionError::Rejected(reason));
        }

        // Compression isn't supported (yet?)
        if handshake.compression_id != CompressionId::None as u8 {
            let reason = format!("Unsupported compression: {}", handshake.compression_id);
            let ack = HandshakeAck::rejected(self.config.server_api_version, &reason);
            self.transport.send_bytes(&ack.to_bytes())?;
            return Err(SessionError::Rejected(reason));
        }

        // Encryption isn't supported (yet?)
        if handshake.encryption_id != EncryptionId::None as u8 {
            let reason = format!("Unsupported encryption: {}", handshake.encryption_id);
            let ack = HandshakeAck::rejected(self.config.server_api_version, &reason);
            self.transport.send_bytes(&ack.to_bytes())?;
            return Err(SessionError::Rejected(reason));
        }

        // We in boys
        let ack = HandshakeAck::accepted(
            self.config.server_api_version,
            handshake.codec_id,
            handshake.compression_id,
            handshake.encryption_id,
        );

        self.transport.send_bytes(&ack.to_bytes())?;
        self.state = SessionState::Active;

        println!(
            "Handshake accepted, client={}, server={}",
            handshake.client_api_version,
            self.config.server_api_version
        );

        Ok(())
    }

    fn recv_and_dispatch(&mut self, registry: &CommandRegistry) -> Result<(), SessionError> {
        let data = self.transport.recv_bytes()?;
        
        let frame: Frame = self.codec.decode(&data).map_err(|e| {
            println!("Failed to decode frame: {e}");
            SessionError::CodecError(e.to_string())
        })?;

        // Handle all kind of message we might get
        match frame {
            Frame::Request {
                call_id,
                namespace,
                command,
                payload,
            } => {
                let cmd_id = CommandId::new(namespace, command);

                let response_frame = Dispatcher::dispatch(registry, call_id, cmd_id, &payload);

                let response_bytes = self.codec.encode(&response_frame).map_err(|e| {
                    SessionError::CodecError(format!("Failed to encode response: {e}"))
                })?;

                self.transport.send_bytes(&response_bytes)?;
            }
            Frame::Ping => {
                let pong_bytes = self
                    .codec
                    .encode(&Frame::Pong)
                    .map_err(|e| SessionError::CodecError(e.to_string()))?;

                self.transport.send_bytes(&pong_bytes)?;
            }
            Frame::Pong => { } // Not much to do here lol
            _ => {
                println!("Unexpected frame type from client, how did this pass all the checks??");
            }
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),
    #[error("Invalid handshake: {0}")]
    InvalidHandshake(String),
    #[error("Handshake rejected: {0}")]
    Rejected(String),
    #[error("Codec error: {0}")]
    CodecError(String),
}
