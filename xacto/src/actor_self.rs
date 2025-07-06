use tokio_util::sync::CancellationToken;

pub struct ActorSelf {
    cancel: CancellationToken,
}

impl ActorSelf {
    pub fn new(cancel: CancellationToken) -> Self {
        Self { cancel }
    }

    pub fn exit(&self) -> () {
        self.cancel.cancel();
    }
}
