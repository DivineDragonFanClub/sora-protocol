use crate::transport::TransportError;

// Useful if you use Tokio for async stuff in the client side
pub trait AsyncTransport: Send {
    fn send_bytes(
        &mut self,
        data: &[u8],
    ) -> impl std::future::Future<Output = Result<(), TransportError>> + Send;

    fn recv_bytes(
        &mut self,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, TransportError>> + Send;
}
