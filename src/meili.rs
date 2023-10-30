use std::error::Error;

use serde_json::json;

use crate::client::Client;
use crate::config::config;

pub(crate) fn index_uid() -> &'static str {
    let config = config().unwrap();
    let index_uid = format!("{}---notes", config.meili.index);
    let index_uid_static = Box::leak(index_uid.into_boxed_str());
    index_uid_static
}

pub(crate) fn url() -> String {
    let config = config().unwrap();
    let protocol = if config.meili.ssl { "https" } else { "http" };
    format!("{}://{}:{}", protocol, config.meili.host, config.meili.port)
}

pub async fn connection(client: &Client) -> Result<(), Box<dyn Error>> {
    let version = client.get(format!("{}/{}", url(), "version")).send().await?;
    let status = version.status();
    if !status.is_success() {
        return Err(format!("Error occurred while connecting to Meilisearch: {}", status).into());
    }
    let value = version.json::<serde_json::Value>().await?;
    let version = value["pkgVersion"].as_str().unwrap();
    Ok(println!("Meilisearch version: {}", version))
}

pub async fn reset(client: &Client) -> Result<(), Box<dyn Error>> {
    let url = url();
    let index = index_uid();

    let delete = client.delete(format!("{}/{}", url, format!("indexes/{}", index)));
    let delete = delete.send().await?;
    if !delete.status().is_success() {
        println!("Error occurred while deleting index: {}, Skipping...", delete.status());
    }
    let response = delete.json::<serde_json::Value>().await?;
    println!("Delete: {}, {}", response["status"], response["taskUid"]);

    let create = client
        .post(format!("{}/{}", url, "indexes"))
        .json(&json!({
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

    let settings = client.patch(format!("{}/{}", url, format!("indexes/{}/settings", index)));
    let settings = settings.json(&update_settings).send().await?;
    let response = settings.json::<serde_json::Value>().await?;
    println!("Settings: {}, {}", response["status"], response["taskUid"]);

    Ok(())
}
