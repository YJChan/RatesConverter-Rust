use serde_json::Value;

pub fn parse_json(data: String) -> Result<Value, serde_json::Error> {
    let json_data: Value = serde_json::from_str(&data)?;
    Ok(json_data)
}

pub async fn fetch_url(uri: String) -> Result<String, reqwest::Error> {
    let body = reqwest::get(&uri).await?.text().await?;

    Ok(body)
}
