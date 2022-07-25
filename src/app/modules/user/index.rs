use deadpool_lapin::{Manager, Pool, PoolError};
use futures::{join, StreamExt};
use lapin::{options::*, types::FieldTable, BasicProperties, ConnectionProperties};
use std::convert::Infallible;
use std::result::Result as StdResult;
use thiserror::Error as ThisError;
use tokio_amqp::*;
use warp::{Filter, Rejection, Reply};
use crate::constants::DB_URL;
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
pub fn with_rmq(pool: Pool) -> impl Filter<Extract = (Pool,), Error = Infallible> + Clone {
    warp::any().map(move || pool.clone())
}
pub async fn add_msg_handler() -> WebResult<impl Reply> {
    let addr = std::env::var("AMQP_ADDR").unwrap_or_else(|_| DB_URL.into());
    let manager = Manager::new(addr, ConnectionProperties::default().with_tokio());
    let pool: Pool = deadpool::managed::Pool::builder(manager)
        .max_size(10)
        .build()
        .expect("can create pool");
    let payload = b"Hello world!";
    let rmq_con = get_rmq_con(pool).await.map_err(|e| {
        eprintln!("can't connect to rmq, {}", e);
        warp::reject::custom(Error::RMQPoolError(e))
    })?;
    let channel = rmq_con.create_channel().await.map_err(|e| {
        eprintln!("can't create channel, {}", e);
        warp::reject::custom(Error::RMQError(e))
    })?;
    channel
        .basic_publish(
            "",
            "hello",
            BasicPublishOptions::default(),
            payload.to_vec(),
            BasicProperties::default(),
        )
        .await
        .map_err(|e| {
            eprintln!("can't publish: {}", e);
            warp::reject::custom(Error::RMQError(e))
        })?
        .await
        .map_err(|e| {
            eprintln!("can't publish: {}", e);
            warp::reject::custom(Error::RMQError(e))
        })?;
    Ok("OK")
}
pub async fn health_handler() -> WebResult<impl Reply> {
    Ok("OK")
}
pub async fn get_rmq_con(pool: Pool) -> RMQResult<Connection> {
    let connection = pool.get().await?;
    Ok(connection)
}