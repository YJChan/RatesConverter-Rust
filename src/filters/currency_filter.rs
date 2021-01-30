use crate::handlers::currency_handler;
use warp::Filter;

pub fn rates () -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let cors = warp::cors()        
    .allow_headers(vec!["User-Agent", "Sec-Fetch-Mode", "Referer", "Origin", "Access-Control-Request-Method", "Access-Control-Request-Headers", "Content-Type", "Accept", "Accept-Encoding", "Cache-Control"])
    .allow_any_origin();
    
    euro_bank_rates()
    .or(daily_rates())
    .or(weekly_rates()).with(cors)
}

pub fn euro_bank_rates() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api"/"exchgrate")
        .and(warp::get())
        .and_then(currency_handler::euro_bank_rates)
}

pub fn daily_rates() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api"/"daily-rates")
        .and(warp::get())
        .and_then(currency_handler::daily_rates)
}

pub fn weekly_rates() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("api"/"weekly-rates"/u8)            
        .and(warp::get())            
        .and_then(move |num:u8| currency_handler::weekly_rates(num))            
}

