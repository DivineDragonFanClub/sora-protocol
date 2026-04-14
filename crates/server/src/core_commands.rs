use sora_protocol::command::{Command, CommandContext};
use sora_protocol::error::RpcError;
use sora_protocol::registry::NamespaceInfo;
use serde::{Deserialize, Serialize};

// This is a command to be used by a gateway server to work around Ryujinx not freeing dangling pointers.
// It tells client the IP and port of the actual server

#[derive(Serialize, Deserialize)]
pub struct ConnectRequest;

#[derive(Serialize, Deserialize)]
pub struct ConnectResponse {
    pub host: String,
    pub port: u16,
}

pub struct ConnectHandler {
    pub host: String,
    pub port: u16,
}

impl Command for ConnectHandler {
    const NAME: &'static str = "connect";
    type Request = ConnectRequest;
    type Response = ConnectResponse;

    fn handle(&self, _ctx: &CommandContext, _request: Self::Request) -> Result<Self::Response, RpcError> {
        Ok(ConnectResponse { host: self.host.clone(), port: self.port })
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListCommandsRequest;

#[derive(Serialize, Deserialize)]
pub struct ListCommandsResponse {
    pub namespaces: Vec<NamespaceInfo>,
}

pub struct ListCommandsHandler;

impl Command for ListCommandsHandler {
    const NAME: &'static str = "list_commands";
    type Request = ListCommandsRequest;
    type Response = ListCommandsResponse;

    fn handle(&self, ctx: &CommandContext, _request: Self::Request) -> Result<Self::Response, RpcError> {
        Ok(ListCommandsResponse { namespaces: ctx.list_commands() })
    }
}

