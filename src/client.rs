use std::ops::Deref;
use reqwest::header::{HeaderMap, HeaderValue};

#[derive(Debug)]
pub struct Client(reqwest::Client);

impl Client {
    pub fn new(token: Option<String>) -> Result<Self, reqwest::Error> {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("application/json"));

        if let Some(apikey) = token {
            headers.insert("Authorization", HeaderValue::from_str(&*format!("Bearer {}", apikey)).unwrap());
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Self(client))
    }
}

impl Deref for Client {
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}