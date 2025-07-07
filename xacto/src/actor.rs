use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{ActorError, ActorSelf};

pub type ActorResult<T = ()> = Result<T, ActorError>;

// (scope_id, actor_task_id)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId(pub u32, pub u32);

#[async_trait]
pub trait Actor: Send + 'static {
    type Args: Send + 'static;
    type Msg: Send + 'static;

    async fn start(this: &ActorSelf<Self>, args: Self::Args) -> ActorResult<Self>
    where
        Self: Sized;

    async fn receive(&mut self, this: &ActorSelf<Self>, msg: Self::Msg) -> ActorResult
    where
        Self: Sized;

    async fn exit(&mut self) -> ActorResult;
}
