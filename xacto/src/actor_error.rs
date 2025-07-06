#[derive(Debug)]
pub enum CallError<M> {
    Send(SendError<M>),
    Recv(RecvError),
}

#[derive(Debug)]
pub enum ActorError {
    Link(LinkError<(), ()>),
    Send(SendError<()>),
    Recv(RecvError),
    Unknown(Box<dyn std::error::Error + Send + 'static>),
}

#[derive(Debug)]
pub enum LinkError<U, M> {
    Unavailable(U),
    Send(SendError<M>),
    Call(CallError<M>),
}

#[derive(Debug)]
pub enum SendError<M> {
    Full(M),
    Closed(M),
}

#[derive(Debug)]
pub enum RecvError {
    Timeout,
    Closed,
}

impl std::fmt::Display for ActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorError::Link(e) => write!(f, "actor link error: {:?}", e),
            ActorError::Send(e) => write!(f, "actor send error: {:?}", e),
            ActorError::Recv(e) => write!(f, "actor recv error: {:?}", e),
            ActorError::Unknown(e) => write!(f, "actor unknown error: {:?}", e),
        }
    }
}

impl<E> From<E> for ActorError
where
    E: std::error::Error + Send + 'static,
{
    fn from(e: E) -> Self {
        ActorError::Unknown(Box::new(e))
    }
}

impl<M> From<SendError<M>> for ActorError {
    fn from(value: SendError<M>) -> Self {
        match value {
            SendError::Full(_) => ActorError::Send(SendError::Full(())),
            SendError::Closed(_) => ActorError::Send(SendError::Closed(())),
        }
    }
}

impl<M> From<CallError<M>> for ActorError {
    fn from(value: CallError<M>) -> Self {
        match value {
            CallError::Send(e) => e.into(),
            CallError::Recv(e) => ActorError::Recv(e),
        }
    }
}
