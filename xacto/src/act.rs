use tokio::sync::{mpsc, oneshot};

use crate::{ActorId, CallError, RecvError, SendError};

#[derive(Debug)]
pub struct Reply<T> {
    tx: oneshot::Sender<T>,
}

impl<T: Send + 'static> Reply<T> {
    pub fn new(tx: oneshot::Sender<T>) -> Self {
        Self { tx }
    }

    pub fn send(self, value: T) -> Result<(), SendError<T>> {
        self.tx.send(value).map_err(|e| SendError::Closed(e))
    }
}

pub enum ActorSignal<M> {
    Msg(M),
}

impl<M> ActorSignal<M> {
    fn unwrap_msg(self) -> M {
        match self {
            ActorSignal::Msg(msg) => msg,
            _ => panic!("expected msg"),
        }
    }
}

pub struct Act<Msg> {
    id: ActorId,
    tx: mpsc::Sender<ActorSignal<Msg>>,
}

impl<Msg> std::fmt::Debug for Act<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Act({:?})", self.id)
    }
}

impl<Msg> Act<Msg> {
    pub fn new(id: ActorId, tx: mpsc::Sender<ActorSignal<Msg>>) -> Self {
        Self { id, tx }
    }

    pub fn id(&self) -> ActorId {
        self.id
    }

    pub fn tx(&self) -> mpsc::Sender<ActorSignal<Msg>> {
        self.tx.clone()
    }

    fn create_signal(&self, msg: Msg) -> ActorSignal<Msg> {
        ActorSignal::Msg(msg)
    }

    pub async fn cast(&self, msg: Msg) -> Result<(), SendError<Msg>> {
        let signal = self.create_signal(msg);
        if let Err(e) = self.tx.send(signal).await {
            return Err(SendError::Closed(e.0.unwrap_msg()));
        }
        Ok(())
    }

    pub fn try_cast(&self, msg: Msg) -> Result<(), SendError<Msg>> {
        let signal = self.create_signal(msg);
        if let Err(e) = self.tx.try_send(signal) {
            return match e {
                mpsc::error::TrySendError::Full(e) => Err(SendError::Full(e.unwrap_msg())),
                mpsc::error::TrySendError::Closed(e) => Err(SendError::Closed(e.unwrap_msg())),
            };
        }
        Ok(())
    }

    #[must_use]
    pub async fn call_manually<T, F>(&self, f: F) -> Result<T, CallError<Msg>>
    where
        T: Send + 'static,
        F: FnOnce(Reply<T>) -> Msg,
    {
        let (tx, rx) = oneshot::channel();
        let msg = f(Reply::new(tx));
        let signal = self.create_signal(msg);

        if let Err(e) = self.tx.send(signal).await {
            return Err(CallError::Send(SendError::Closed(e.0.unwrap_msg())));
        }

        match rx.await {
            Ok(result) => Ok(result),
            Err(_) => Err(CallError::Recv(RecvError::Closed)),
        }
    }

    #[must_use]
    pub async fn call<T, F>(&self, f: F) -> Result<T, CallError<Msg>>
    where
        T: Send + 'static,
        F: FnOnce(Reply<T>) -> Msg,
    {
        let (tx, rx) = oneshot::channel();
        let msg = f(Reply::new(tx));
        let signal = self.create_signal(msg);

        if let Err(e) = self.tx.send(signal).await {
            return Err(CallError::Send(SendError::Closed(e.0.unwrap_msg())));
        }

        match rx.await {
            Ok(result) => Ok(result),
            Err(_) => Err(CallError::Recv(RecvError::Closed)),
        }
    }

    #[must_use]
    pub async fn try_call<T, F>(&self, f: F) -> Result<T, CallError<Msg>>
    where
        T: Send + 'static,
        F: FnOnce(Reply<T>) -> Msg,
    {
        let (tx, rx) = oneshot::channel();
        let msg = f(Reply::new(tx));
        let signal = self.create_signal(msg);

        if let Err(e) = self.tx.try_send(signal) {
            return match e {
                mpsc::error::TrySendError::Full(e) => {
                    Err(CallError::Send(SendError::Full(e.unwrap_msg())))
                }
                mpsc::error::TrySendError::Closed(e) => {
                    Err(CallError::Send(SendError::Closed(e.unwrap_msg())))
                }
            };
        }

        match rx.await {
            Ok(result) => Ok(result),
            Err(_) => Err(CallError::Recv(RecvError::Closed)),
        }
    }
}

impl<Msg> Clone for Act<Msg> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            tx: self.tx.clone(),
        }
    }
}
