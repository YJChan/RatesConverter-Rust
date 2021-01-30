#[macro_use]
extern crate diesel;
extern crate dotenv;

mod filters;
mod handlers;
mod services;
mod db;

use filters::currency_filter;
use warp::Filter;
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();

    let api = currency_filter::rates();

    let routes = api.with(warp::log("RATES"));

    let port = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or_else(|| 8000);

    println!("Listening on port 0.0.0.0: {}", port);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}
