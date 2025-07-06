use anymap::AnyMap;
use slab::Slab;

use crate::act::Reply;

pub struct ReplyMap {
    reply_slabs: AnyMap,
}

impl ReplyMap {
    pub fn new() -> Self {
        Self {
            reply_slabs: AnyMap::new(),
        }
    }

    pub fn get_reply<T: 'static>(&mut self, id: usize) -> Option<Reply<T>> {
        let slab = self.reply_slabs.get_mut::<Slab<Reply<T>>>()?;
        slab.try_remove(id)
    }

    pub fn insert_reply<T: 'static>(&mut self, reply: Reply<T>) -> usize {
        let slab = match self.reply_slabs.entry::<Slab<Reply<T>>>() {
            anymap::Entry::Occupied(entry) => entry.into_mut(),
            anymap::Entry::Vacant(entry) => entry.insert(Slab::new()),
        };
        slab.insert(reply)
    }
}
