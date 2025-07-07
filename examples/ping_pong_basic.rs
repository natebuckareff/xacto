use async_trait::async_trait;
use xacto::{Act, Actor, ActorResult, ActorSelf, Scope, ScopeContext};

#[derive(Debug)]
enum Msg {
    Ping(Act<Msg>),
    Pong(Act<Msg>),
}

struct Ping {
    id: i32,
    count: i32,
}

#[async_trait]
impl Actor for Ping {
    type Msg = Msg;
    type Args = i32;

    async fn start(_: &ActorSelf<Self>, id: Self::Args) -> ActorResult<Self> {
        println!("Actor {} started", id);
        Ok(Self { id, count: 0 })
    }

    async fn receive(&mut self, this: &ActorSelf<Self>, msg: Self::Msg) -> ActorResult {
        println!("Actor {} received message {:?}", self.id, &msg);

        let act = this.act().clone();
        let (from, reply) = match msg {
            Msg::Ping(from) => (from, Msg::Pong(act)),
            Msg::Pong(from) => (from, Msg::Ping(act)),
        };

        if let Err(e) = from.cast(reply).await {
            println!("Actor {} failed to send: {:?}", self.id, e);
        }

        self.count += 1;

        if self.count > 5 {
            this.exit();
        }

        Ok(())
    }

    async fn exit(&mut self) -> ActorResult {
        println!("Actor {} will exit", self.id);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> ActorResult {
    let context = ScopeContext::new();
    let mut scope = Scope::new(context);

    let ping1 = scope.spawn::<Ping>(1).await;
    let ping2 = scope.spawn::<Ping>(2).await;

    ping1.cast(Msg::Ping(ping2.clone())).await?;
    ping2.cast(Msg::Ping(ping1.clone())).await?;

    while let Some((id, output)) = scope.next_finished().await {
        println!("Finished: {:?}, {:?}", id, output);
    }

    scope.exit_and_wait().await;

    Ok(())
}
