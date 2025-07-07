use std::sync::{Arc, RwLock};

use tokio::sync::watch;

use crate::{Act, LinkError, Reply};

pub struct LinkPublisher<Msg> {
    tx: watch::Sender<Option<Act<Msg>>>,
}

impl<Msg> LinkPublisher<Msg> {
    pub fn new() -> (Self, Link<Msg>) {
        let (tx, rx) = watch::channel(None);
        let writer = Self { tx };
        let link = Link::new(rx);
        (writer, link)
    }

    pub fn update(&self, act: Act<Msg>) {
        let _ = self.tx.send(Some(act));
    }
}

#[derive(Clone)]
pub struct Link<Msg> {
    rx: watch::Receiver<Option<Act<Msg>>>,
    local: Arc<RwLock<Option<Act<Msg>>>>,
}

impl<Msg> Link<Msg> {
    pub fn new(rx: watch::Receiver<Option<Act<Msg>>>) -> Self {
        Self {
            rx,
            local: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_with_msg(&self, msg: Msg) -> Result<(Msg, Act<Msg>), LinkError<Msg, Msg>> {
        match self.get().await {
            Ok(act) => Ok((msg, act)),
            Err(e) => match e {
                LinkError::Unavailable(()) => Err(LinkError::Unavailable(msg)),
                LinkError::Send(send_error) => Err(LinkError::Send(send_error)),
                LinkError::Call(call_error) => Err(LinkError::Call(call_error)),
            },
        }
    }

    pub async fn get(&self) -> Result<Act<Msg>, LinkError<(), Msg>> {
        if let Ok(true) = self.rx.has_changed() {
            let mut rx = self.rx.clone();
            if let Some(act) = rx.borrow_and_update().as_ref() {
                let mut local = self.local.write().map_err(|e| {
                    eprintln!("lock poisoned: {e:?}");
                    LinkError::Unavailable(())
                })?;
                *local = Some(act.clone());
                return Ok(act.clone());
            }
        }

        {
            let cached = self.local.read().map_err(|e| {
                eprintln!("lock poisoned: {e:?}");
                LinkError::Unavailable(())
            })?;

            if let Some(act) = cached.as_ref() {
                return Ok(act.clone());
            }
        }

        let mut rx = self.rx.clone();
        let result = rx.wait_for(|act| act.is_some()).await;

        let act = match result {
            Ok(act) => act.as_ref().unwrap().clone(),
            Err(_) => return Err(LinkError::Unavailable(())),
        };

        let mut local = self.local.write().map_err(|e| {
            eprintln!("lock poisoned: {e:?}");
            LinkError::Unavailable(())
        })?;

        *local = Some(act.clone());

        Ok(act)
    }

    pub async fn cast(&self, msg: Msg) -> Result<(), LinkError<Msg, Msg>> {
        match self.get_with_msg(msg).await {
            Ok((msg, act)) => act.cast(msg).await.map_err(|e| LinkError::Send(e)),
            Err(e) => Err(e),
        }
    }

    pub async fn try_cast(&self, msg: Msg) -> Result<(), LinkError<Msg, Msg>> {
        match self.get_with_msg(msg).await {
            Ok((msg, act)) => act.try_cast(msg).map_err(|e| LinkError::Send(e)),
            Err(e) => Err(e),
        }
    }

    pub async fn call<T, F>(&self, f: F) -> Result<T, LinkError<(), Msg>>
    where
        T: Send + 'static,
        F: FnOnce(Reply<T>) -> Msg,
    {
        match self.get().await {
            Ok(act) => act.call(f).await.map_err(|e| LinkError::Call(e)),
            Err(e) => Err(e),
        }
    }

    pub async fn try_call<T, F>(&self, f: F) -> Result<T, LinkError<(), Msg>>
    where
        T: Send + 'static,
        F: FnOnce(Reply<T>) -> Msg,
    {
        match self.get().await {
            Ok(act) => act.try_call(f).await.map_err(|e| LinkError::Call(e)),
            Err(e) => Err(e),
        }
    }
}
