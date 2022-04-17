use std::sync::{Mutex};
use actix::prelude::*;
use actix_web::{get, web, App, HttpServer, Responder};
use std::time::Duration;
use sqlx::postgres::PgPoolOptions;
use sqlx::Postgres;
use sqlx::Pool;
use chrono::{DateTime, Local};

struct MyActor {
    counter: web::Data<Mutex<MyCounter>>,
    pool: Pool<Postgres>,
}

impl Actor for MyActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        println!("MyActor is started");

        ctx.run_interval(Duration::from_secs(10), move |this, _ctx| {
            let pool = this.pool.clone();
            let wd_counter = this.counter.clone();
            // let c1 = c.lock().unwrap().increment();
            // println!("c1 {}", c1);

            let ab = Arbiter::new();
            ab.spawn(async move {
                let r: (DateTime<Local>,) = sqlx::query_as("SELECT current_timestamp")
                        .fetch_one(&pool).await.expect("Failed to select current_timestamp");

                println!("current timestamp is {}", r.0);

                let value = wd_counter.lock().unwrap().increment();
                println!("current counter is {}", value);
            });
        });

    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        println!("MyActor is stopped");
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
    format!("current counter is {}", value)
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {

    let c = MyCounter::new(0);
    let mx_counter = Mutex::new(c);
    let wd_counter = web::Data::new(mx_counter);
    
    let connection_pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(3)
        .connect("postgres://postgres:password@localhost:5434/postgres")
        .await
        .expect("Failed to connect to Postgres");

    MyActor { counter: wd_counter.clone(), pool: connection_pool.clone() }.start();

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