use anymap::{Map, any::Any};
use slab::Slab;

use crate::Reply;

type SendAnyMap = Map<dyn Any + Send>;

pub struct ReplyMap {
    reply_slabs: SendAnyMap,
}

impl ReplyMap {
    pub fn new() -> Self {
        Self {
            reply_slabs: SendAnyMap::new(),
        }
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
