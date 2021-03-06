use actix::prelude::*;
use actix_daemon_utils::{
    graceful_stop::GracefulStop,
    looper::{Looper, Task},
};
use actix_postgres::{bb8_postgres::tokio_postgres::tls::NoTls, PostgresActor, PostgresMessage};

struct MyActor {
    msg: String,
    seconds: u64,
    pg: Addr<PostgresActor<NoTls>>,
}

impl Actor for MyActor {
    type Context = Context<Self>;
}

impl Handler<Task> for MyActor {
    type Result = u64;

    fn handle(&mut self, _msg: Task, ctx: &mut Self::Context) -> Self::Result {
        println!("{}", self.msg);
        let task = PostgresMessage::new(|pool| {
            Box::pin(async move {
                let connection = pool.get().await?;
                connection
                    .query("SELECT NOW()::TEXT as c", &vec![])
                    .await
                    .map_err(|err| err.into())
            })
        });
        let msg2 = self.msg.clone();
        self.pg
            .send(task)
            .into_actor(self)
            .map(move |res, _act, _ctx| match res {
                Ok(res) => match res {
                    Ok(res) => {
                        let val: &str = res[0].get(0);
                        println!("{},{}", msg2, val);
                    }
                    Err(err) => println!("{:?}", err),
                },
                Err(err) => println!("{:?}", err),
            })
            .wait(ctx);
        self.seconds
    }
}

struct MyActor2 {
    msg: String,
    seconds: u64,
    pg: Addr<PostgresActor<NoTls>>,
}

impl Actor for MyActor2 {
    type Context = Context<Self>;
}

impl Handler<Task> for MyActor2 {
    type Result = u64;

    fn handle(&mut self, _msg: Task, ctx: &mut Self::Context) -> Self::Result {
        println!("{}", self.msg);
        let task = PostgresMessage::new(|pool| {
            Box::pin(async move {
                let connection = pool.get().await?;
                connection
                    .query_one("SELECT NOW()::TEXT as c", &vec![])
                    .await
                    .map_err(|err| err.into())
            })
        });
        let msg2 = self.msg.clone();
        self.pg
            .send(task)
            .into_actor(self)
            .map(move |res, _act, _ctx| match res {
                Ok(res) => match res {
                    Ok(res) => {
                        let val: &str = res.get(0);
                        println!("{},{}", msg2, val);
                    }
                    Err(err) => println!("{:?}", err),
                },
                Err(err) => println!("{:?}", err),
            })
            .wait(ctx);
        self.seconds
    }
}

fn main() {
    let path = std::env::var("PG_PATH").unwrap();
    let sys = actix::System::new("main");
    let graceful_stop = GracefulStop::new();
    let pg_actor = PostgresActor::start(&path, NoTls).unwrap();
    let actor1 = MyActor {
        msg: "x".to_string(),
        seconds: 1,
        pg: pg_actor.clone(),
    }
    .start();
    let looper1 = Looper::new(actor1.recipient(), graceful_stop.clone_system_terminator()).start();
    let actor2 = MyActor2 {
        msg: "y".to_string(),
        seconds: 1,
        pg: pg_actor,
    }
    .start();
    let looper2 = Looper::new(actor2.recipient(), graceful_stop.clone_system_terminator()).start();
    graceful_stop
        .subscribe(looper1.recipient())
        .subscribe(looper2.recipient())
        .start();

    let _ = sys.run();
    println!("main terminated");
}
