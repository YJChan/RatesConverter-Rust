#![deny(warnings)]
extern crate diesel;
extern crate warp_currency;

use warp::Filter;
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();

    let api = filters::rates();

    let routes = api.with(warp::log("RATES"));

    let port = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or_else(|| 8000);

    println!("Listening on port 0.0.0.0: {}", port);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}

mod filters {
    use super::handlers;    
    use warp::{Filter};
    
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
            .and_then(handlers::euro_bank_rates)
    }

    pub fn daily_rates() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api"/"daily-rates")
            .and(warp::get())
            .and_then(handlers::daily_rates)
    }

    pub fn weekly_rates() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api"/"weekly-rates")
            .and(warp::get())
            .and_then(handlers::weekly_rates)
    }
}


mod handlers {
    use warp::{reject, Rejection, Reply};
    use serde::{Serialize, Deserialize};
    use super::services::{common_service, rate_service};
    use std::{env, vec};
    use dotenv::dotenv;
    use warp_currency::models::{NewRate};
    use std::collections::HashMap;
    use chrono::{Duration, Utc};
    
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
        
        let euro_exchange_uri = String::from("http://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml");
        let euro_bank_rates = common_service::fetch_url(euro_exchange_uri).await.unwrap();

        Ok(warp::reply::with_header(euro_bank_rates, "Content-Type", "text/xml"))
    }

    pub async fn daily_rates() -> Result<impl Reply, Rejection> {
        dotenv().ok();
        let api_key = env::var("FIXER_API_KEY").expect("Missing api key");
        let api_endpoint = format!("http://data.fixer.io/api/latest?access_key={}&format=1", &api_key);

        if rate_service::exist_today_rate() {
            let today_rate = rate_service::find_today_rate();            
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
            rate_service::batch_insert_rate(&daily_data);

            Ok(warp::reply::json(&json_data))

        }
    }

    pub async fn weekly_rates() -> Result<impl Reply, Rejection> {
        let number_of_days = 14;
        let today_dt = format!("{}", Utc::now().format("%Y-%m-%d"));
        let days_from_now = format!("{}", (Utc::now() - Duration::days(number_of_days)).format("%Y-%m-%d"));
        let seven_day_rates = rate_service::find_one_week_rates(&today_dt, &days_from_now);
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
    
}

pub mod services {
    pub mod common_service {
        use serde_json::Value;

        pub fn parse_json(data: String) -> Result<Value, serde_json::Error> {
            let json_data: Value = serde_json::from_str(&data)?;
            Ok(json_data)
        }

        pub async fn fetch_url(uri: String) -> Result<String, reqwest::Error> {
            let body = reqwest::get(&uri).await?.text().await?;

            Ok(body)
        }
    }

    pub mod rate_service {
        use diesel::prelude::*;
        use warp_currency::establish_connection;
        use warp_currency::models::*;
        use warp_currency::schema::tb_rates::dsl::*;
        use chrono::{Utc};
        use diesel::sql_query;
        use diesel::sql_types::Text;

        pub fn insert_rate(rate: NewRate) -> bool {
            let conn = establish_connection();
            let rows_inserted = diesel::insert_into(tb_rates)
                .values(rate)
                .execute(&conn)
                .expect("Unable to insert new rate");
            if rows_inserted == 1 {
                true
            } else {
                false
            }
        }

        pub fn batch_insert_rate(rates: &Vec<NewRate>) -> usize {
            let conn = establish_connection();
            diesel::insert_into(tb_rates)
                .values(rates)
                .execute(&conn)
                .expect("Unable to batch insert rates")
        }

        pub fn exist_today_rate() -> bool {
            let today_dt = format!("{}", Utc::now().format("%Y-%m-%d"));

            let conn = establish_connection();
            let count: i64 = tb_rates
                .count()
                .filter(rate_dt.eq(today_dt))
                .get_result(&conn).unwrap();
            
            // println!("count is {}", count);
            if count > 1 {
                true
            } else {
                false
            }
        }

        pub fn find_today_rate() -> (String, Vec<Rates>) {
            let today_dt = format!("{}", Utc::now().format("%Y-%m-%d"));
            let conn = establish_connection();
            let rates: Vec<Rates> = tb_rates
                .filter(rate_dt.eq(&today_dt))
                .load::<Rates>(&conn).unwrap();
            
            (today_dt, rates)
        }

        pub fn find_one_week_rates(today_dt: &str, days_from_now: &str) -> Vec<Rates> {
            let conn = establish_connection();            
            let query = sql_query(format!("select * from tb_rates where DATE(rate_dt) between DATE($1) AND DATE($2) ORDER BY rate_dt desc"));            
            
            let query_result: Vec<Rates> = query
            .bind::<Text, _>(days_from_now)
            .bind::<Text, _>(today_dt)
            .load::<Rates>(&conn).unwrap();

            query_result
        }
    }
}
