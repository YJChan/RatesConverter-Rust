use std::vec;
use tokio::sync::mpsc::Sender;
use bytes::Bytes;
use select::document::Document;
use select::predicate::{Class, Attr, Name};
use regex::Regex;
use itertools::Itertools;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveCurrencyRate {
    bid: String,
    ask: String,
    high: String,
    low: String
}

pub async fn scrape_by_url(url: &str, tx: Sender<Bytes>) {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build().unwrap();
    let resp = client.get(url).send().await.unwrap();

    let resp_text = resp.text().await.unwrap();
    let doc = Document::from_read(resp_text.as_bytes()).unwrap();

    let rate_table_nodes = doc.find(Class("crossRatesTbl")).next().unwrap();
    let rgx = Regex::new(r#"pair_\d+"#).unwrap();
    let mut pairs: Vec<String> = vec![];    
    let rate_panel_html_text = rate_table_nodes.inner_html();
    
    for mtch in rgx.find_iter(&rate_panel_html_text) {
        pairs.push(rate_panel_html_text[mtch.start()..mtch.end()].to_string());
    }
    if pairs.len() > 0 {
        pairs = pairs.into_iter().unique().collect();
    }
    
    let mut rate_map: HashMap<String, LiveCurrencyRate> = HashMap::new();

    for pair in pairs {
        let cls_name: &str = &pair;
        let cls_index = cls_name.split("_").collect::<Vec<&str>>()[1];        
        let node = doc.find(Attr("id", cls_name)).next().unwrap();
        let bid_cls_name: &str = &format!("pid-{}-bid", cls_index);
        let ask_cls_name: &str = &format!("pid-{}-ask", cls_index);
        let high_cls_name: &str = &format!("pid-{}-high", cls_index);
        let low_cls_name: &str = &format!("pid-{}-low", cls_index);        

        let bid_node = node.find(Class(bid_cls_name)).next().unwrap();
        let name_node = node.find(Name("a")).next().unwrap();
        let ask_node = node.find(Class(ask_cls_name)).next().unwrap();
        let high_node = node.find(Class(high_cls_name)).next().unwrap();
        let low_node = node.find(Class(low_cls_name)).next().unwrap();

        rate_map.insert(name_node.text(), LiveCurrencyRate {
            bid: bid_node.text(),
            ask: ask_node.text(),
            high: high_node.text(),
            low: low_node.text()
        });        
    }
    
    // println!("{:?}", rate_map);
    
    tokio::spawn(async move {        
        tx.send(Bytes::from(serde_json::to_string(&rate_map).unwrap())).await.unwrap();
    });

}


#[cfg(test)]
mod rate_scraper_test {
    use tokio::sync::mpsc;
    use bytes::Bytes;
    use dotenv::dotenv;
    #[tokio::test]
    async fn it_works() {
        dotenv().ok();
        let live_rate_url = std::env::var("LIVE_RATE_SCRAPE_URL").expect("Missing live rate url");
        let (tx, mut rx) = mpsc::channel::<Bytes>(8);
        
        super::scrape_by_url(&live_rate_url, tx.clone()).await;

        rx.recv().await;
    }
}