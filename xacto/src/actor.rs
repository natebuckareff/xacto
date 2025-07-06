use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{act::Act, actor_error::ActorError, actor_self::ActorSelf};

pub type ActorResult<T = ()> = Result<T, ActorError>;

// (scope_id, actor_task_id)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId(pub u32, pub u32);

#[async_trait]
pub trait Actor: Send + 'static {
    type Args: Send + 'static;
    type Msg: Send + 'static;

    async fn start(act: &Act<Self::Msg>, args: Self::Args) -> ActorResult<Self>
    where
        Self: Sized;

    async fn receive(&mut self, this: &ActorSelf, msg: Self::Msg) -> ActorResult;
    async fn exit(&mut self) -> ActorResult;
}
