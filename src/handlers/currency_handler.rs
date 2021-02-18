use sse::Event;
use tokio::sync::oneshot;
// use tokio::sync::Mutex;
#[allow(unused_imports)]
use tokio::sync::{mpsc, mpsc::error::RecvError};
use warp::{reject, Rejection, Reply};
use warp::sse;
use serde::{Serialize, Deserialize};
use crate::services::{common_service, currency_service};
use std::{convert::Infallible, env, vec};
use dotenv::dotenv;
use super::super::db::models::{NewRate};
use std::collections::HashMap;
use chrono::{Duration, Utc};
use futures::{stream::iter, Stream};
use tokio_stream::{wrappers::ReceiverStream};

#[derive(Debug)]
struct RequestError;

impl reject::Reject for RequestError {}

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

#[derive(Serialize, Deserialize)]
struct RatesMap {
    success: bool,
    timestamp: i64,
    base: String,
    date: String,
    rates: HashMap<String, f32>
}

#[derive(Serialize, Deserialize)]
struct RateOfDate {
    rates: HashMap<String, f32>,
    date: String 
}

#[derive(Serialize, Deserialize)]
struct SevenDayRatesMap {
    success: bool,
    timestamp: i64,
    base: String,
    date: String,
    rates: Vec<RateOfDate>
}

pub async fn euro_bank_rates() -> Result<impl Reply, warp::Rejection> {
    dotenv().ok();
    let euro_exchange_url = env::var("EURO_RATE_URL").expect("Missing euro rate url");    
    let euro_bank_rates = common_service::fetch_url(euro_exchange_url).await.unwrap();

    Ok(warp::reply::with_header(euro_bank_rates, "Content-Type", "text/xml"))
}

pub async fn daily_rates() -> Result<impl Reply, Rejection> {
    dotenv().ok();
    let daily_rates_url = env::var("DAILY_RATE_URL").expect("Missing daily rate url");
    let api_key = env::var("FIXER_API_KEY").expect("Missing api key");
    let api_endpoint = format!("{}{}&format=1", &daily_rates_url, &api_key);    

    if currency_service::exist_today_rate() {
        let today_rate = currency_service::find_today_rate();            
        let mut rates: HashMap<String, f32> = HashMap::new();
        for rate in &today_rate.1 {
            let currency = rate.currency.clone();
            rates.insert(currency, rate.conversion_rate);
        }

        let rate_map = RatesMap {
            success: true,
            timestamp: Utc::now().timestamp_millis(),
            base: "EUR".to_string(),
            date: today_rate.0,
            rates: rates
        };

        Ok(warp::reply::json(&rate_map))

    } else {
        let resp = common_service::fetch_url(api_endpoint).await.unwrap();
        
        let json_data = common_service::parse_json(resp).unwrap();
        
        let mut daily_data: Vec<NewRate> = Vec::new();
        let rates = json_data["rates"].as_object().unwrap();
        for (key, val) in rates {
            // println!("{} : {}", key, val);
            let conversion_rate = val.as_f64().unwrap() as f32;
            let today: String = String::from(json_data["date"].as_str().unwrap());
            let base_currency = env::var("BASE_CURR").expect("Missing base currency");
            
            let new_rate = NewRate {
                rate_dt: today,
                base: base_currency,
                currency: String::from(key),
                conversion_rate: conversion_rate,
            };
            
            // println!("{}, {}, {}, {}", new_rate.rate_dt, new_rate.base, new_rate.currency, new_rate.conversion_rate);
            daily_data.push(new_rate);
        }   
        currency_service::batch_insert_rate(&daily_data);

        Ok(warp::reply::json(&json_data))

    }
}

pub async fn weekly_rates(mut number_of_days:u8) -> Result<impl Reply, Rejection> {
    if number_of_days > 90 {
        number_of_days = 90;
    }
    let today_dt = format!("{}", Utc::now().format("%Y-%m-%d"));
    let days_from_now = format!("{}", (Utc::now() - Duration::days(number_of_days as i64)).format("%Y-%m-%d"));
    let seven_day_rates = currency_service::find_one_week_rates(&today_dt, &days_from_now);
    let mut rates_collection:Vec<RateOfDate> = vec![];
    
    let mut dt = Utc::now() + Duration::days(1);
    let rates_len = seven_day_rates.len();
    let mut cursor = 0;
    for _ in 0..number_of_days {
        dt = dt - Duration::days(1);
        let dt_str = format!("{}", dt.format("%Y-%m-%d"));
        let mut rates: HashMap<String, f32> = HashMap::new();
        
        for rate in seven_day_rates[cursor..rates_len].into_iter() {
            if rate.rate_dt == dt_str {
                let currency = rate.currency.clone();
                rates.insert(currency, rate.conversion_rate);
                cursor += 1;
            } else {                    
                let rate_of_date = RateOfDate {
                    date: dt_str.to_string(),
                    rates: rates.clone()
                };
                &rates_collection.push(rate_of_date);                    
                break;
            }
        }
    }

    let rates_map = SevenDayRatesMap {
        success: true,
        timestamp: Utc::now().timestamp_millis(),
        base: "EUR".to_string(),
        date: format!("{} - {}", days_from_now, today_dt),
        rates: rates_collection
    };

    Ok(warp::reply::json(&rates_map))
}

//Result<impl Reply<Stream<Item = Result<Event, RecvError>>>, Rejection> 
pub async fn live_rates() -> Result<impl Reply, Rejection> {
    dotenv().ok();
    let live_rate_url = env::var("LIVE_RATE_SCRAPE_URL").expect("Missing live rate url");
    let (tx, rx) = oneshot::channel::<String>();
    // let rx = Arc::new(Mutex::new(rx));
    // let mut rate_map: HashMap<String, rate_scraper::LiveCurrencyRate> = HashMap::new();
    
    rate_scraper::scrape_by_url(live_rate_url, tx).await;    
    // let rate_stream = ReceiverStream::new(rx);
    // let event_stream = rate_stream.map(move |b| {
    //     sse_rate_event(b)
    // });
    // Ok(sse::reply(sse::keep_alive().interval(std::time::Duration::from_secs(1)).stream(event_stream)))
    let rates = rx.await.unwrap();
    // let f = future::ok::<_, String>(b);
    // let event_stream = f.into_stream();
    Ok(sse::reply(sse_rate_event(rates)))
    // tx.send("ssss".to_string()).unwrap();
    // Ok(sse::reply(
    //     sse::keep_alive()
    //     .interval(std::time::Duration::from_secs(300))
    //     .stream(            
    //         tx2.subscribe().into_stream().map(|msg| {
    //             msg.map(|data| {
    //                 Event::default()
    //                 .id(1.to_string())
    //                 .data(data)
    //                 .event("message")                    
    //                 .retry(std::time::Duration::from_millis(10000))                                                                                    
    //             })
    //         })
    //     )
    // ))
}

fn sse_rate_event(b: String) -> impl Stream<Item = Result<Event, Infallible>> {
    iter(vec![
        Ok(
            sse::Event::default()
            .data(&b)
            .event("message")
            .retry(std::time::Duration::from_millis(1000))        
        )
    ])
}