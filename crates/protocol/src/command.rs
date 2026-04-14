use core::fmt;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::RpcError;
use crate::registry::{CommandRegistry, NamespaceInfo};

/// Namespace 0 = core
/// 1 to 255 = reserved for game specific commands
/// 256+ = plugin 
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CommandId {
    pub namespace: u16,
    pub command: u16,
}

impl CommandId {
    pub const fn new(namespace: u16, command: u16) -> Self {
        Self { namespace, command }
    }

    pub const fn core(command: u16) -> Self {
        Self {
            namespace: 0,
            command,
        }
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.namespace, self.command)
    }
}

pub mod core_commands {
    use super::CommandId;

    pub const LIST_COMMANDS: CommandId = CommandId::core(0x00);
    pub const SUBSCRIBE: CommandId = CommandId::core(0x01);
    pub const UNSUBSCRIBE: CommandId = CommandId::core(0x02);
    pub const CONNECT: CommandId = CommandId::core(0x03);
}

/// Context passed to handlers so they can consult the registry.
/// Heavily inspired by Bevy's asset server.
pub struct CommandContext<'a> {
    pub registry: &'a CommandRegistry,
}

impl<'a> CommandContext<'a> {
    pub fn new(registry: &'a CommandRegistry) -> Self {
        Self { registry }
    }

    pub fn list_commands(&self) -> Vec<NamespaceInfo> {
        self.registry.list()
    }
}

pub trait Command: Send + Sync + 'static {
    const NAME: &'static str;

    type Request: Serialize + DeserializeOwned;
    type Response: Serialize + DeserializeOwned;

    fn handle(&self, ctx: &CommandContext, request: Self::Request) -> Result<Self::Response, RpcError>;
}

// Needed because the associated types of Command make it so each instance is considered a different type
pub trait ErasedCommand: Send + Sync {
    fn name(&self) -> &'static str;
    fn handle_erased(&self, ctx: &CommandContext, params: &[u8]) -> Result<Vec<u8>, RpcError>;
}
