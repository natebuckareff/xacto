use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    act::{Act, ActorSignal},
    actor::{Actor, ActorId},
    actor_error::ActorError,
    actor_self::ActorSelf,
};

pub type ActorTaskResult = Result<(), ActorTaskError>;

pub enum ActorTaskError {
    Start(ActorError),
    Receive(ActorError),
    Exit(ActorError),
}

pub struct ActorTask<A: Actor> {
    id: ActorId,
    rx: mpsc::Receiver<ActorSignal<A::Msg>>,
    cancel: CancellationToken,
}

impl<A: Actor> ActorTask<A> {
    pub fn new(
        id: ActorId,
        rx: mpsc::Receiver<ActorSignal<A::Msg>>,
        cancel: CancellationToken,
    ) -> Self {
        Self { id, rx, cancel }
    }

    pub async fn run(mut self, act: Act<A::Msg>, args: A::Args) -> ActorTaskResult {
        let mut actor = A::start(&act, args).await.map_err(ActorTaskError::Start)?;
        let actor_self = ActorSelf::new(self.cancel.clone());

        loop {
            tokio::select! {
                signal = self.rx.recv() => {
                    self.handle_signal(&mut actor, &actor_self, signal).await?;
                }
                _ = self.cancel.cancelled() => {
                    self.handle_cancel(&mut actor).await?;
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_signal(
        &mut self,
        actor: &mut A,
        actor_self: &ActorSelf,
        signal: Option<ActorSignal<A::Msg>>,
    ) -> ActorTaskResult {
        match signal {
            Some(signal) => {
                match signal {
                    ActorSignal::Msg(msg) => {
                        actor
                            .receive(actor_self, msg)
                            .await
                            .map_err(ActorTaskError::Receive)?;
                    }
                };
            }
            None => self.cancel.cancel(),
        }
        Ok(())
    }

    async fn handle_cancel(&mut self, actor: &mut A) -> ActorTaskResult {
        self.rx.close();
        actor.exit().await.map_err(ActorTaskError::Exit)?;
        Ok(())
    }
}
