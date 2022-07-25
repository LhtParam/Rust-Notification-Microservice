#![allow(unused_variables)]
use actix_web::Responder;
use actix_web::{get, post, web, web::Json, HttpRequest, HttpResponse};
use futures::future::ok;
#[path = "../app/modules/user/index.rs"]
mod user_controller;
#[get("/")]
pub async fn index() -> HttpResponse {
    HttpResponse::Ok().body("Rust microservice alive!")
}
#[post("/publish_msg")]
pub async fn send_msg() -> HttpResponse {
    let response = user_controller::add_msg_handler().await;
    HttpResponse::Ok().body("Message published...")
}
// let health_route = warp::path!("health").and_then(health_handler);
// let add_msg_route = warp::path!("msg")
//         .and(warp::post())
//         .and(with_rmq(pool.clone()))
//         .and_then(add_msg_handler);
//     let routes = health_route.or(add_msg_route);