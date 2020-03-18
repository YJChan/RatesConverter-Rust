use serde::{Deserialize, Serialize};
use super::schema::tb_rates;

#[derive(Queryable, Deserialize, Serialize, Clone)]
pub struct Rates {
    pub id: i32,
    pub rate_dt: String,
    pub base: String,
    pub currency: String,
    pub conversion_rate: f32,
}

#[derive(Insertable)]
#[table_name = "tb_rates"]
pub struct NewRate {
    pub rate_dt: String,
    pub base: String,
    pub currency: String,
    pub conversion_rate: f32,
}