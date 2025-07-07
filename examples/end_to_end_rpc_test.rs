use async_trait::async_trait;
use tokio::sync::oneshot;
use xacto::{Actor, ActorError, ActorResult, ActorSelf, Reply};
use xacto_derive::RpcMessage;

struct MyActor {
    state: u16,
}

#[derive(Debug, RpcMessage)]
enum MyActorMsg {
    Increment,
    Decrement,
    DoSomething(String),
    GetCount(Reply<u16>),
    GetCount2(String, Reply<u16>),
    GetCount3(String, i32, Reply<String>),
}

#[derive(Debug, RpcMessage)]
enum AnotherMsg {
    DoSomething(String),
    DoSomeCall(String),
}

#[async_trait]
impl Actor for MyActor {
    type Msg = MyActorMsg;
    type Args = u16;

    async fn start(_: &ActorSelf<Self>, args: Self::Args) -> ActorResult<Self> {
        println!("MyActor started");
        Ok(Self { state: args })
    }

    async fn receive(&mut self, _: &ActorSelf<Self>, msg: Self::Msg) -> ActorResult {
        println!("MyActor receiving messages");

        match msg {
            MyActorMsg::Increment => {
                self.state += 1;
            }
            MyActorMsg::Decrement => {
                self.state -= 1;
            }
            MyActorMsg::DoSomething(msg) => {
                println!("DoSomething: {}", msg);
            }
            MyActorMsg::GetCount(reply) => {
                reply.send(self.state)?;
            }
            MyActorMsg::GetCount2(name, reply) => {
                println!("GetCount2: {}", name);
                reply.send(self.state)?;
            }
            MyActorMsg::GetCount3(name, count, reply) => {
                println!("GetCount3: {} {}", name, count);
                reply.send(format!(">>>>> {} {}", name, count))?;
            }
        }

        Ok(())
    }

    async fn exit(&mut self) -> ActorResult {
        println!("MyActor exiting");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), ActorError> {
    use xacto::*;

    let context = ScopeContext::new();
    let mut scope = Scope::new(context);

    let mut reply_map = ReplyMap::new();
    let act = scope.spawn::<MyActor>(100).await;

    let (rx, json1) = {
        let (rx, env) =
            reply_map.create_request(|reply| MyActorMsg::GetCount3("hello".into(), 1000, reply));

        (rx, serde_json::to_string(&env).unwrap())
    };

    println!("json1: {}", &json1);

    let json2 = {
        let act_clone = act.clone();
        let env = serde_json::from_str(&json1).unwrap();
        let res = MyActorMsg::proxy_request(env, move |msg| Some((msg, act_clone)))
            .await
            .unwrap()
            .unwrap();

        serde_json::to_string(&res).unwrap()
    };

    println!("json2: {}", &json2);

    {
        let env = serde_json::from_str(&json2).unwrap();
        reply_map.handle_response::<MyActorMsg>(env).await.unwrap();
    }

    {
        dbg!(rx.await.unwrap());
    }

    scope.exit_and_wait().await;

    Ok(())
}
