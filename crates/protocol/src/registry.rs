use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::codec::Codec;
use crate::command::{Command, CommandContext, CommandId, ErasedCommand};
use crate::error::RpcError;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Namespace already registered: {0}")]
    NamespaceAlreadyExists(String),
    #[error("Namespace id {0} already taken")]
    NamespaceIdTaken(u16),
    #[error("Namespace id {0} is out of range (must be 1..=255)")]
    NamespaceIdOutOfRange(u16),
    #[error("Command name '{name}' already registered at {existing}")]
    DuplicateCommandName { name: String, existing: CommandId },
    #[error("Namespace {0} not found")]
    NamespaceNotFound(u16),
    #[error("No more namespace IDs available")]
    NamespaceExhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceInfo {
    pub id: u16,
    pub name: String,
    pub commands: Vec<CommandInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    pub id: u16,
    pub name: String,
}

// Wraps a Command with a codec so it can be stored as a type-erased handler.
pub(crate) struct CommandWrapper<Cmd, Cdc> {
    handler: Cmd,
    codec: Cdc,
}

impl<Cmd: Command, Cdc: Codec> ErasedCommand for CommandWrapper<Cmd, Cdc> {
    fn name(&self) -> &'static str {
        Cmd::NAME
    }

    fn handle_erased(&self, ctx: &CommandContext, params: &[u8]) -> Result<Vec<u8>, RpcError> {
        let request: Cmd::Request = self.codec.decode(params).map_err(|e| {
            RpcError::invalid_params(format!("Failed to deserialize request: {e}"))
        })?;

        let response = self.handler.handle(ctx, request)?;

        self.codec.encode(&response).map_err(|e| {
            RpcError::command_failed(format!("Failed to serialize response: {e}"))
        })
    }
}

struct Namespace {
    name: String,
    handlers: BTreeMap<u16, Box<dyn ErasedCommand>>, // We store ErasedCommand here because it allows us to treat them as the same type.
    next_command_id: u16,
}

pub struct CommandRegistry {
    namespaces: BTreeMap<u16, Namespace>,
    namespace_names: BTreeMap<String, u16>,
    command_names: BTreeMap<String, CommandId>,
    next_plugin_namespace: u16,
}

impl CommandRegistry {
    // NOTE: This might go unused, the idea is that one day plugins might register their own namespace.
    // When that happens they'll be stored at index 256 and beyond.
    const PLUGIN_NAMESPACE_START: u16 = 256;

    pub fn new() -> Self {
        let mut registry = Self {
            namespaces: BTreeMap::new(),
            namespace_names: BTreeMap::new(),
            command_names: BTreeMap::new(),
            next_plugin_namespace: Self::PLUGIN_NAMESPACE_START,
        };

        registry.namespaces.insert(
            0,
            Namespace {
                name: "core".to_string(),
                handlers: BTreeMap::new(),
                next_command_id: 0,
            },
        );
        registry.namespace_names.insert("core".to_string(), 0);

        registry
    }

    pub fn register_namespace(&mut self, name: impl Into<String>) -> Result<u16, RegistryError> {
        let name = name.into();

        if self.namespace_names.contains_key(&name) {
            return Err(RegistryError::NamespaceAlreadyExists(name));
        }

        let id = self.next_plugin_namespace;

        if id == u16::MAX {
            return Err(RegistryError::NamespaceExhausted);
        }
        self.next_plugin_namespace += 1;

        self.namespaces.insert(
            id,
            Namespace {
                name: name.clone(),
                handlers: BTreeMap::new(),
                next_command_id: 0,
            },
        );
        self.namespace_names.insert(name, id);

        Ok(id)
    }
    
    pub fn register_namespace_at(
        &mut self,
        id: u16,
        name: impl Into<String>,
    ) -> Result<u16, RegistryError> {
        if id == 0 || id >= Self::PLUGIN_NAMESPACE_START {
            return Err(RegistryError::NamespaceIdOutOfRange(id));
        }
        let name = name.into();
        if self.namespace_names.contains_key(&name) {
            return Err(RegistryError::NamespaceAlreadyExists(name));
        }
        if self.namespaces.contains_key(&id) {
            return Err(RegistryError::NamespaceIdTaken(id));
        }

        self.namespaces.insert(
            id,
            Namespace {
                name: name.clone(),
                handlers: BTreeMap::new(),
                next_command_id: 0,
            },
        );
        self.namespace_names.insert(name, id);

        Ok(id)
    }

    pub fn register_command<Cmd: Command, Cdc: Codec + Clone + 'static>(
        &mut self,
        namespace_id: u16,
        handler: Cmd,
        codec: &Cdc,
    ) -> Result<CommandId, RegistryError> {
        let namespace = self
            .namespaces
            .get(&namespace_id)
            .ok_or(RegistryError::NamespaceNotFound(namespace_id))?;

        let command_id = namespace.next_command_id;
        self.register_command_at(namespace_id, command_id, handler, codec)
    }

    // For core commands that need a fixed ID.
    pub fn register_command_at<Cmd: Command, Cdc: Codec + Clone + 'static>(
        &mut self,
        namespace_id: u16,
        command_id: u16,
        handler: Cmd,
        codec: &Cdc,
    ) -> Result<CommandId, RegistryError> {
        let name = Cmd::NAME;

        if let Some(existing) = self.command_names.get(name) {
            return Err(RegistryError::DuplicateCommandName {
                name: name.to_string(),
                existing: *existing,
            });
        }

        let namespace = self
            .namespaces
            .get_mut(&namespace_id)
            .ok_or(RegistryError::NamespaceNotFound(namespace_id))?;

        let full_id = CommandId::new(namespace_id, command_id);

        let wrapper = CommandWrapper {
            handler,
            codec: codec.clone(),
        };

        namespace.handlers.insert(command_id, Box::new(wrapper));
        self.command_names.insert(name.to_string(), full_id);
        
        if command_id >= namespace.next_command_id {
            namespace.next_command_id = command_id + 1;
        }

        Ok(full_id)
    }

    pub fn get_handler(&self, id: &CommandId) -> Option<&dyn ErasedCommand> {
        self.namespaces
            .get(&id.namespace)
            .and_then(|ns| ns.handlers.get(&id.command))
            .map(|h| h.as_ref())
    }

    pub fn get_handler_by_name(
        &self,
        command_name: &str,
    ) -> Option<(&dyn ErasedCommand, CommandId)> {
        let id = self.command_names.get(command_name)?;
        let handler = self.get_handler(id)?;
        Some((handler, *id))
    }

    pub fn namespace_id(&self, name: &str) -> Option<u16> {
        self.namespace_names.get(name).copied()
    }

    pub fn list(&self) -> Vec<NamespaceInfo> {
        self.namespaces
            .iter()
            .map(|(id, ns)| NamespaceInfo {
                id: *id,
                name: ns.name.clone(),
                commands: ns
                    .handlers
                    .iter()
                    .map(|(cmd_id, handler)| CommandInfo {
                        id: *cmd_id,
                        name: handler.name().to_string(),
                    })
                    .collect(),
            })
            .collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}