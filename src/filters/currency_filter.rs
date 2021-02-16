use crate::handlers::currency_handler;
use warp::{Filter, Reply, Rejection, filters::BoxedFilter};
use warp::get;

pub fn rates () -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone{
    let cors = warp::cors()        
    .allow_headers(vec!["User-Agent", "Sec-Fetch-Mode", "Referer", "Origin", "Access-Control-Request-Method", "Access-Control-Request-Headers", "Content-Type", "Accept", "Accept-Encoding", "Cache-Control"])
    .allow_any_origin();
        
    euro_bank_rates()
    .or(daily_rates())
    .or(weekly_rates())
    .or(live_rates())
    .with(cors)
    .with(warp::log("WARP-CURRENCY"))
}

pub fn euro_bank_rates() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api"/"exchgrate")
        .and(get())
        .and_then(currency_handler::euro_bank_rates)
}

pub fn daily_rates() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api"/"daily-rates")
        .and(get())
        .and_then(currency_handler::daily_rates)
}

pub fn weekly_rates() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api"/"weekly-rates"/u8)            
        .and(get())            
        .and_then(move |num:u8| currency_handler::weekly_rates(num))            
}

pub fn live_rates() -> BoxedFilter<(impl Reply,)> {
    warp::path!("api"/"live-rates")
        .and(get())
        .and_then(move || {            
            currency_handler::live_rates()
        })
        .boxed()
}