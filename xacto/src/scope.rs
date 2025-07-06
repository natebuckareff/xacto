use std::{
    collections::HashMap,
    panic::AssertUnwindSafe,
    sync::{Arc, Mutex},
};

use futures_util::FutureExt;
use tokio::{
    sync::mpsc,
    task::{AbortHandle, JoinError, JoinSet},
};
use tokio_util::sync::CancellationToken;

use crate::{
    act::Act,
    actor::{Actor, ActorId},
    actor_task::{ActorTask, ActorTaskError},
};

pub struct ScopeContext {
    next_scope_id: u32,
    cancel: CancellationToken,
}

impl ScopeContext {
    pub fn new() -> Arc<Mutex<Self>> {
        let cancel = CancellationToken::new();
        Arc::new(Mutex::new(Self {
            next_scope_id: 0,
            cancel,
        }))
    }
}

struct ActorState {
    handle: AbortHandle,
    cancel: CancellationToken,
}

pub enum ActorOutput {
    Success,
    Failed(ActorTaskError),
    Aborted,
    Panicked(Option<Box<dyn std::any::Any + Send>>),
    Unknown(JoinError),
}

pub struct Scope {
    context: Arc<Mutex<ScopeContext>>,
    id: u32,
    next_actor_id: u32,
    join_set: JoinSet<ActorOutput>,
    task_ids: HashMap<tokio::task::Id, ActorId>,
    actors: HashMap<ActorId, ActorState>,
    cancel: CancellationToken,
}

impl Scope {
    pub fn new(context: Arc<Mutex<ScopeContext>>) -> Self {
        let (id, cancel) = {
            let mut context = context.lock().unwrap();
            let id = context.next_scope_id;
            context.next_scope_id += 1;
            let cancel = context.cancel.child_token();
            (id, cancel)
        };

        Self {
            context,
            id,
            next_actor_id: 0,
            join_set: JoinSet::new(),
            task_ids: HashMap::new(),
            actors: HashMap::new(),
            cancel,
        }
    }

    pub fn child_scope(&mut self) -> Scope {
        let id = {
            let mut context = self.context.lock().unwrap();
            let id = context.next_scope_id;
            context.next_scope_id += 1;
            id
        };
        let cancel = self.cancel.child_token();
        Self {
            context: self.context.clone(),
            id,
            next_actor_id: 0,
            join_set: JoinSet::new(),
            task_ids: HashMap::new(),
            actors: HashMap::new(),
            cancel,
        }
    }

    pub async fn spawn<A: Actor>(&mut self, args: A::Args) -> Act<A::Msg> {
        assert!(!self.cancel.is_cancelled(), "scope cancelled");

        let id = ActorId(self.id, self.next_actor_id);
        self.next_actor_id += 1;

        let (tx, rx) = mpsc::channel(100);
        let act = Act::new(id, tx);
        let cancel = self.cancel.child_token();

        let task = ActorTask::<A>::new(id, rx, cancel.clone());
        let task = task.run(act.clone(), args);

        let handle = self.join_set.spawn(async move {
            match AssertUnwindSafe(task).catch_unwind().await {
                Ok(result) => match result {
                    Ok(()) => ActorOutput::Success,
                    Err(e) => ActorOutput::Failed(e),
                },
                Err(e) => ActorOutput::Panicked(Some(e)),
            }
        });

        self.task_ids.insert(handle.id(), id);
        self.actors.insert(id, ActorState { handle, cancel });

        act
    }

    pub fn is_running(&self, id: ActorId) -> bool {
        self.actors.contains_key(&id)
    }

    pub fn exit_actor(&mut self, id: ActorId) {
        if let Some(state) = self.actors.get_mut(&id) {
            state.handle.abort();
        }
    }

    pub fn abort_actor(&mut self, id: ActorId) {
        if let Some(state) = self.actors.get_mut(&id) {
            state.handle.abort();
        }
    }

    fn __cleanup_actor_state(&mut self, task_id: tokio::task::Id) -> (ActorId, ActorState) {
        let id = self.task_ids.remove(&task_id).expect("task id not found");
        let state = self.actors.remove(&id).expect("actor not found");
        (id, state)
    }

    pub async fn next_finished(&mut self) -> Option<(ActorId, ActorOutput)> {
        if let Some(result) = self.join_set.join_next_with_id().await {
            let output = match result {
                Ok((task_id, output)) => {
                    let (id, _) = self.__cleanup_actor_state(task_id);
                    (id, output)
                }
                Err(e) => {
                    let (id, _) = self.__cleanup_actor_state(e.id());

                    if e.is_cancelled() {
                        (id, ActorOutput::Aborted)
                    } else if e.is_panic() {
                        (id, ActorOutput::Panicked(None))
                    } else {
                        (id, ActorOutput::Unknown(e))
                    }
                }
            };
            Some(output)
        } else {
            None
        }
    }

    pub async fn exit_all(&mut self) {
        self.cancel.cancel();
    }

    pub async fn exit_and_wait(&mut self) {
        self.cancel.cancel();
        while let Some(_) = self.next_finished().await {}
    }

    pub fn abort_all(&mut self) {
        self.join_set.abort_all();
    }

    pub async fn hard_shutdown(&mut self) {
        self.join_set.shutdown().await;
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        self.abort_all();
    }
}
