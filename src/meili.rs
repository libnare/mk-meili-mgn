use std::error::Error;

use reqwest::{Client, RequestBuilder};
use serde_json::json;

use crate::config::config;

pub(crate) async fn index_uid() -> Result<&'static str, Box<dyn Error>> {
    let config = config()?;
    let index_uid = format!("{}---notes", config.meili.index);
    let index_uid_static = Box::leak(index_uid.into_boxed_str());
    Ok(index_uid_static)
}

pub(crate) async fn url() -> Result<String, Box<dyn Error>> {
    let config = config()?;
    let protocol = if config.meili.ssl { "https" } else { "http" };
    Ok(format!("{}://{}:{}", protocol, config.meili.host, config.meili.port))
}

pub(crate) async fn get_request_builder(
    url: &str,
    path: &str,
    method: reqwest::Method,
) -> Result<RequestBuilder, Box<dyn Error>> {
    let mut request_builder = Client::new()
        .request(method, format!("{}/{}", url, path));

    let config = config()?;
    if let Some(apikey) = &config.meili.apikey {
        request_builder = request_builder.header("Authorization", format!("Bearer {}", apikey));
    }

    Ok(request_builder)
}

pub async fn connection() -> Result<(), Box<dyn Error>> {
    let version = get_request_builder(
        &url().await?,
        "version",
        reqwest::Method::GET,
    ).await?;
    let response = version.send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Error occurred while connecting to Meilisearch: {}", status).into());
    }
    let value = response.json::<serde_json::Value>().await?;
    let version = value["pkgVersion"].as_str().unwrap();
    Ok(println!("Meilisearch version: {}", version))
}

pub async fn reset() -> Result<(), Box<dyn Error>> {
    let url = url().await.unwrap();
    let index = index_uid().await.unwrap();

    let delete = get_request_builder(
        &url,
        format!("indexes/{}", index).as_str(),
        reqwest::Method::DELETE,
    ).await?;
    let delete = delete.send().await?;
    if !delete.status().is_success() {
        println!("Error occurred while deleting index: {}, Skipping...", delete.status());
    }
    let response = delete.json::<serde_json::Value>().await?;
    println!("Delete: {}, {}", response["status"], response["taskUid"]);

    let create = get_request_builder(
        &url,
        "indexes",
        reqwest::Method::POST,
    ).await?;
    let create = create.json(&json!({
        "uid": index,
        "primaryKey": "id",
    })).send().await?;
    let response = create.json::<serde_json::Value>().await?;
    println!("Create: {}, {}", response["status"], response["taskUid"]);


    let update_settings = json!({
        "searchableAttributes": [
            "text",
            "cw",
        ],
        "sortableAttributes": [
            "createdAt",
        ],
        "filterableAttributes": [
            "createdAt",
            "userId",
            "userHost",
            "channelId",
            "tags",
        ],
        "typoTolerance": {
            "enabled": false
        },
        "pagination": {
            "maxTotalHits": 10000
        }
    });

    let settings = get_request_builder(
        &url,
        format!("indexes/{}/settings", index).as_str(),
        reqwest::Method::PATCH,
    ).await?;
    let settings = settings.json(&update_settings).send().await?;
    let response = settings.json::<serde_json::Value>().await?;
    println!("Settings: {}, {}", response["status"], response["taskUid"]);

    Ok(())
}
