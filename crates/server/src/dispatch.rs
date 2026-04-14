use sora_protocol::command::{CommandContext, CommandId};
use sora_protocol::error::RpcError;
use sora_protocol::frame::Frame;
use sora_protocol::registry::CommandRegistry;

pub struct Dispatcher;

impl Dispatcher {
    pub fn dispatch(
        registry: &CommandRegistry,
        call_id: u32,
        cmd_id: CommandId,
        payload: &[u8],
    ) -> Frame {
        let handler = match registry.get_handler(&cmd_id) {
            Some(h) => h,
            None => {
                let err = RpcError::command_not_found(cmd_id.namespace, cmd_id.command);
                return Frame::error_from_rpc(call_id, &err);
            }
        };

        let ctx = CommandContext::new(registry);
        let result = handler.handle_erased(&ctx, payload);

        match result {
            Ok(response_bytes) => Frame::response(call_id, response_bytes),
            Err(err) => Frame::error_from_rpc(call_id, &err),
        }
    }
}
