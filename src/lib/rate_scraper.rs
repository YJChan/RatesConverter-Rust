use std::vec;
use tokio::sync::{oneshot::Sender};
use select::document::Document;
use select::predicate::{Class, Attr, Name};
use regex::Regex;
use itertools::Itertools;
use serde::{Serialize, Deserialize};
use reqwest::header;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveCurrencyRate {
    pair: String,
    bid: f64,
    ask: f64,
    high: f64,
    low: f64
}

impl Drop for LiveCurrencyRate {
    fn drop(&mut self) {
        println!("Dropped {}", self.pair);
    }
}

struct ScrapedData {
    html_text: String,
    doc: Document
}

impl ScrapedData {
    pub fn new(html_text:String) -> ScrapedData {        
        let doc = Document::from_read(html_text.as_bytes()).unwrap();
        ScrapedData {
            html_text, 
            doc,
        }
    }
}

impl Drop for ScrapedData {
    fn drop(&mut self) {
        println!("Scraped data is dropped");
    }
}

pub async fn scrape_by_url(url: String, tx: Sender<String>) {    
    let mut resp_text = String::new();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let mut pairs: Vec<String> = vec![];
    let mut rate_vec: Vec<LiveCurrencyRate> = vec![];
        
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/88.0.4324.150 Safari/537.36")
        .build().unwrap();
    
    let resp = client.get(&url).send().await.unwrap();
    resp_text = resp.text().await.unwrap_or("".to_string());
        
    // println!("{:?}", client);
    if resp_text.len() == 0 {
        tokio::spawn(async move {        
            tx.send("".to_string()).unwrap();        
        });
    } else {        
        let scraped_data = ScrapedData::new(resp_text);
        // let doc = Document::from_read(resp_text.as_bytes()).unwrap();
    
        if scraped_data.html_text.len() > 0 {
            let rate_table_nodes = scraped_data.doc.find(Class("crossRatesTbl")).next().unwrap();
            let rgx = Regex::new(r#"pair_\d+"#).unwrap();     
            {
                let rate_panel_html_text = rate_table_nodes.inner_html();
                
                for mtch in rgx.find_iter(&rate_panel_html_text) {
                    pairs.push(rate_panel_html_text[mtch.start()..mtch.end()].to_string());
                }
            }
        }

        if pairs.len() > 0 {
            pairs = pairs.into_iter().unique().collect();
        }
        // println!("{:?}", rate_vec);

        for pair in pairs {
            let cls_name: &str = &pair;
            let cls_index = cls_name.split("_").collect::<Vec<&str>>()[1];        
            let node = scraped_data.doc.find(Attr("id", cls_name)).next().unwrap();
            let bid_cls_name: &str = &format!("pid-{}-bid", cls_index);
            let ask_cls_name: &str = &format!("pid-{}-ask", cls_index);
            let high_cls_name: &str = &format!("pid-{}-high", cls_index);
            let low_cls_name: &str = &format!("pid-{}-low", cls_index);        

            let bid_node = node.find(Class(bid_cls_name)).next().unwrap();
            let name_node = node.find(Name("a")).next().unwrap();
            let ask_node = node.find(Class(ask_cls_name)).next().unwrap();
            let high_node = node.find(Class(high_cls_name)).next().unwrap();
            let low_node = node.find(Class(low_cls_name)).next().unwrap();

            
            rate_vec.push(LiveCurrencyRate {
                pair: name_node.text(),
                bid: bid_node.text().parse::<f64>().unwrap_or(-1f64),
                ask: ask_node.text().parse::<f64>().unwrap_or(-1f64),
                high: high_node.text().parse::<f64>().unwrap_or(-1f64),
                low: low_node.text().parse::<f64>().unwrap_or(-1f64)
            });        
        }
        
        // println!("{:?}", rate_vec);
        
        tokio::spawn(async move {        
            tx.send(serde_json::to_string(&rate_vec).unwrap()).unwrap();        
        });
    }
}


#[cfg(test)]
mod rate_scraper_test {
    use tokio::sync::oneshot;
    use dotenv::dotenv;    

    #[tokio::test]
    async fn it_works() {
        dotenv().ok();
        let live_rate_url = std::env::var("LIVE_RATE_SCRAPE_URL").expect("Missing live rate url");
        let (tx, rx) = oneshot::channel::<String>();
        
        super::scrape_by_url(live_rate_url, tx).await;

        rx.await.unwrap();
    }
}