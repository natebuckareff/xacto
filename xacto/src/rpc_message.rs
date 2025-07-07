use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Act, ReplyMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcEnvelope<T> {
    pub id: usize,
    pub payload: T,
}

#[async_trait]
pub trait RpcMessage {
    type Request;
    type Response;

    fn into_request(self, proxy: &mut ReplyMap) -> RpcEnvelope<Self::Request>;

    async fn proxy_request<F: Send>(
        env: RpcEnvelope<Self::Request>,
        f: F,
    ) -> Result<Option<RpcEnvelope<Self::Response>>, ()>
    where
        F: FnOnce(Self) -> Option<(Self, Act<Self>)>,
        Self: Sized;

    async fn proxy_response(
        env: RpcEnvelope<Self::Response>,
        proxy: &mut ReplyMap,
    ) -> Result<(), ()>;
}
