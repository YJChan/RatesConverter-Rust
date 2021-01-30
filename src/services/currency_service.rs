#![allow(dead_code)]
use diesel::prelude::*;
use super::super::db::establish_connection;
use super::super::db::models::*;
use super::super::db::schema::tb_rates::dsl::*;
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
