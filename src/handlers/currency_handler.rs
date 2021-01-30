use warp::{reject, Rejection, Reply};
use serde::{Serialize, Deserialize};
use crate::services::{common_service, currency_service};
use std::{env, vec};
use dotenv::dotenv;
use super::super::db::models::{NewRate};
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

pub async fn weekly_rates(number_of_days:u8) -> Result<impl Reply, Rejection> {
    if number_of_days > 31 {
        
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

