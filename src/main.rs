use std::fmt::Error;
use std::sync::Mutex;
use actix::clock::sleep;
use actix::prelude::*;
use actix_web::{get, web, App, HttpServer, Responder};
use std::time::Duration;
use sqlx::postgres::PgPoolOptions;
use sqlx::Postgres;
use sqlx::Pool;
use chrono::{DateTime, Local};
use log::{debug};

struct MyActor {
    counter: web::Data<Mutex<MyCounter>>,
    pool: Pool<Postgres>,
}

impl Actor for MyActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        debug!("MyActor is started");

        let ab = Arbiter::new();

        ctx.run_interval(Duration::from_secs(10), move |this, _ctx| {
            debug!("Inside run_interval");

            let pool = this.pool.clone();
            let wd_counter = this.counter.clone();
            // let c1 = c.lock().unwrap().increment();
            // println!("c1 {}", c1);

             ab.spawn(async move {
                let r: (DateTime<Local>,) = sqlx::query_as("select current_timestamp from sym_channel where channel_id = 'channel1' for update")
                        .fetch_one(&pool).await.expect("Failed to select current_timestamp");

                debug!("current timestamp is {}", r.0);

                let value = wd_counter.lock().unwrap().increment();
                debug!("current counter is {}", value);
            });
            
        });

    }


    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("MyActor is stopped");
    }
}

impl StreamHandler<String> for MyActor {
    fn handle(&mut self, item: String, _ctx: &mut Self::Context) {
        println!("## handled ## {}", item);
    }
}

struct MyCounter(i32);

impl MyCounter {
    fn new(c: i32) -> MyCounter {
        MyCounter(c)
    }

    fn get_value(&self) -> i32 {
        self.0
    }

    fn increment(&mut self) -> i32 {
        self.0 = self.0 + 1;
        self.0
    }
}

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}

#[get("/counter")]
async fn counter(counter: web::Data<Mutex<MyCounter>>) -> String {
    let value: i32 = counter.lock().unwrap().get_value();
    debug!("rest api counter is {}", value);
    format!("rest api counter is {}", value)
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {

    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    let c = MyCounter::new(0);
    let mx_counter = Mutex::new(c);
    let wd_counter = web::Data::new(mx_counter);
    
    let connection_pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(3)
        .connect("postgres://postgres:password@localhost:5434/db2")
        .await
        .expect("Failed to connect to Postgres");

    // MyActor { counter: wd_counter.clone(), pool: connection_pool.clone() }.start();
    let wd_clone = wd_counter.clone();
    let pool_clone = connection_pool.clone();
    let ab = Arbiter::new();
    ab.spawn(async move {
        loop {
            let r: (DateTime<Local>,) = sqlx::query_as("select current_timestamp from sym_channel where channel_id = 'channel1' for update")
                    .fetch_one(&pool_clone).await.expect("Failed to select current_timestamp");

            debug!("current timestamp is {}", r.0);

            let value = wd_clone.lock().unwrap().increment();
            debug!("current counter is {}", value);
            sleep(Duration::from_secs(10)).await;
        }
    });


     HttpServer::new(move || {
        App::new()
            .route("/hello", web::get().to(|| async { "Hello World!" }))
            .service(greet)
            .service(counter)
            .app_data(connection_pool.clone())
            .app_data(wd_counter.clone())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}