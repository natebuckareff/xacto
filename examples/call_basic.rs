use std::time::{Duration, Instant};

use async_trait::async_trait;
use xacto::{Act, Actor, ActorResult, ActorSelf, Reply, Scope, ScopeContext, call};

enum TimeServiceMsg {
    GetTime(Reply<Duration>),
    Exit,
}

struct TimeService {
    start: Instant,
}

#[async_trait]
impl Actor for TimeService {
    type Args = ();
    type Msg = TimeServiceMsg;

    async fn start(_: &ActorSelf<Self>, _: Self::Args) -> ActorResult<Self> {
        Ok(Self {
            start: Instant::now(),
        })
    }

    async fn receive(&mut self, this: &ActorSelf<Self>, msg: Self::Msg) -> ActorResult {
        match msg {
            TimeServiceMsg::GetTime(reply) => {
                let time = self.start.elapsed();
                reply.send(time)?;
            }
            TimeServiceMsg::Exit => {
                this.exit();
            }
        }
        Ok(())
    }

    async fn exit(&mut self) -> ActorResult {
        println!("TimeService exiting");
        Ok(())
    }
}

struct Client;

#[async_trait]
impl Actor for Client {
    type Args = Act<TimeServiceMsg>;
    type Msg = ();

    async fn start(this: &ActorSelf<Self>, svc: Self::Args) -> ActorResult<Self> {
        for _ in 0..10 {
            let time = call!(svc, TimeServiceMsg::GetTime).await?;
            println!("Received time: {}ms", time.as_millis());
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        svc.cast(TimeServiceMsg::Exit).await?;

        this.exit();

        Ok(Self)
    }

    async fn receive(&mut self, _: &ActorSelf<Self>, _: Self::Msg) -> ActorResult {
        Ok(())
    }

    async fn exit(&mut self) -> ActorResult {
        println!("TimeService exiting");
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let context = ScopeContext::new();
    let mut scope = Scope::new(context);

    let svc = scope.spawn::<TimeService>(()).await;
    scope.spawn::<Client>(svc).await;

    while let Some((id, output)) = scope.next_finished().await {
        println!("Task {:?} finished with output {:?}", id, output);
    }

    scope.exit_and_wait().await;
}
