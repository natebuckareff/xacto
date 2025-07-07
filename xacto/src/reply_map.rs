use anymap::{Map, any::Any};
use slab::Slab;
use tokio::sync::oneshot;

use crate::{Reply, RpcEnvelope, RpcMessage};

type SendAnyMap = Map<dyn Any + Send>;

pub struct ReplyMap {
    reply_slabs: SendAnyMap,
}

impl ReplyMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_request<F, M, T>(
        &mut self,
        f: F,
    ) -> (oneshot::Receiver<T>, RpcEnvelope<M::Request>)
    where
        F: FnOnce(Reply<T>) -> M,
        M: RpcMessage,
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let reply = Reply::new(tx);
        let msg = f(reply);
        let env = msg.into_request(self);
        (rx, env)
    }

    pub async fn handle_response<M>(&mut self, env: RpcEnvelope<M::Response>) -> Result<(), ()>
    where
        M: RpcMessage,
    {
        M::proxy_response(env, self).await
    }

    pub fn get_reply<T: Send + 'static>(&mut self, id: usize) -> Option<Reply<T>> {
        let slab = self.reply_slabs.get_mut::<Slab<Reply<T>>>()?;
        slab.try_remove(id)
    }

    pub fn insert_reply<T: Send + 'static>(&mut self, reply: Reply<T>) -> usize {
        let slab = match self.reply_slabs.entry::<Slab<Reply<T>>>() {
            anymap::Entry::Occupied(entry) => entry.into_mut(),
            anymap::Entry::Vacant(entry) => entry.insert(Slab::new()),
        };
        slab.insert(reply)
    }
}

impl Default for ReplyMap {
    fn default() -> Self {
        Self {
            reply_slabs: SendAnyMap::new(),
        }
    }
}
