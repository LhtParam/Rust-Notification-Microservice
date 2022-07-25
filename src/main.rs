#![allow(dead_code)]
use actix_web::{web, App, HttpServer};
use constants::DB_URL;
use deadpool_lapin::{Manager, Pool, PoolError};
use futures::{join, StreamExt};
use lapin::{options::*, types::FieldTable, BasicProperties, ConnectionProperties};
use std::convert::Infallible;
use std::result::Result as StdResult;
use std::time::Duration;
use thiserror::Error as ThisError;
use tokio_amqp::*;
use warp::{Filter, Rejection, Reply};
use crate::modules::get_rmq_con;
type WebResult<T> = StdResult<T, Rejection>;
type RMQResult<T> = StdResult<T, PoolError>;
type Result<T> = StdResult<T, Error>;
type Connection = deadpool::managed::Object<deadpool_lapin::Manager>;
#[derive(ThisError, Debug)]
enum Error {
    #[error("rmq error: {0}")]
    RMQError(#[from] lapin::Error),
    #[error("rmq pool error: {0}")]
    RMQPoolError(#[from] PoolError),
}
impl warp::reject::Reject for Error {}
#[path = "app/constants/index.rs"]
mod constants;
#[path = "app/models/user.rs"]
pub(crate) mod model;
#[path = "app/modules/user/index.rs"]
pub(crate) mod modules;
#[path = "routes/index.rs"]
mod routes;
pub fn init(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("")
            .service(web::scope("/routes"))
            .service(routes::index)
            .service(routes::send_msg),
    );
}
// #[actix_web::main]
// async fn main() -> std::io::Result<()> {
//     let addr =
//         std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://rmq:rmq@127.0.0.1:5672/%2f".into());
//     let manager = Manager::new(addr, ConnectionProperties::default().with_tokio());
//     let pool: Pool = deadpool::managed::Pool::builder(manager)
//         .max_size(10)
//         .build()
//         .expect("can create pool");
//     println!("Started server at localhost:8000");
//     let _ = join!(
//         warp::serve(routes).run(([0, 0, 0, 0], 8000)),
//         rmq_listen(pool.clone())
//     );
//     Ok(())
// }
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let addr = std::env::var("AMQP_ADDR").unwrap_or_else(|_| DB_URL.into());
    let manager = Manager::new(addr, ConnectionProperties::default().with_tokio());
    let pool: Pool = deadpool::managed::Pool::builder(manager)
        .max_size(10)
        .build()
        .expect("can create pool");
    HttpServer::new(move || {
        App::new()
            .app_data(rmq_listen(pool.clone()))
            .configure(init)
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await
}
async fn rmq_listen(pool: Pool) -> Result<()> {
    let mut retry_interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        retry_interval.tick().await;
        println!("connecting rmq consumer...");
        match init_rmq_listen(pool.clone()).await {
            Ok(_) => println!("rmq listen returned"),
            Err(e) => eprintln!("rmq listen had an error: {}", e),
        };
    }
}
async fn init_rmq_listen(pool: Pool) -> Result<()> {
    let rmq_con = get_rmq_con(pool).await.map_err(|e| {
        eprintln!("could not get rmq con: {}", e);
        e
    })?;
    let channel = rmq_con.create_channel().await?;
    let queue = channel
        .queue_declare(
            "hello",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;
    println!("Declared queue {:?}", queue);
    let mut consumer = channel
        .basic_consume(
            "hello",
            "my_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;
    println!("rmq consumer connected, waiting for messages");
    while let Some(delivery) = consumer.next().await {
        if let Ok((channel, delivery)) = delivery {
            println!("received msg: {:?}", delivery);
            channel
                .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                .await?
        }
    }
    Ok(())
}
