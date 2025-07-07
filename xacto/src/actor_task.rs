use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{Act, Actor, ActorError, ActorSignal};

pub type ActorTaskResult = Result<(), ActorTaskError>;

pub struct ActorSelf<A: Actor> {
    act: Act<A::Msg>,
    rx: mpsc::Receiver<ActorSignal<A::Msg>>,
    cancel: CancellationToken,
}

impl<A: Actor> ActorSelf<A> {
    pub fn new(
        act: Act<A::Msg>,
        rx: mpsc::Receiver<ActorSignal<A::Msg>>,
        cancel: CancellationToken,
    ) -> Self {
        Self { act, rx, cancel }
    }

    pub fn act(&self) -> &Act<A::Msg> {
        &self.act
    }

    pub fn exit(&self) -> () {
        self.cancel.cancel();
    }
}

#[derive(Debug)]
pub enum ActorTaskError {
    Start(ActorError),
    Receive(ActorError),
    Exit(ActorError),
}

pub struct ActorTask<A: Actor> {
    this: ActorSelf<A>,
}

impl<A: Actor> ActorTask<A> {
    pub fn new(this: ActorSelf<A>) -> Self {
        Self { this }
    }

    pub async fn run(mut self, args: A::Args) -> ActorTaskResult {
        let mut actor = A::start(&self.this, args)
            .await
            .map_err(ActorTaskError::Start)?;

        loop {
            tokio::select! {
                signal = self.this.rx.recv() => {
                    self.handle_signal(&mut actor, signal).await?;
                }
                _ = self.this.cancel.cancelled() => {
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
        signal: Option<ActorSignal<A::Msg>>,
    ) -> ActorTaskResult {
        match signal {
            Some(signal) => {
                match signal {
                    ActorSignal::Msg(msg) => {
                        actor
                            .receive(&self.this, msg)
                            .await
                            .map_err(ActorTaskError::Receive)?;
                    }
                };
            }
            None => self.this.cancel.cancel(),
        }
        Ok(())
    }

    async fn handle_cancel(&mut self, actor: &mut A) -> ActorTaskResult {
        self.this.rx.close();
        actor.exit().await.map_err(ActorTaskError::Exit)?;
        Ok(())
    }
}
