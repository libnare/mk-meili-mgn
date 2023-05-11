use meilisearch_sdk::Client;
use meilisearch_sdk::settings::PaginationSetting;
use meilisearch_sdk::task_info::TaskInfo;
use serde_json::json;
use std::error::Error;
use crate::config::config;

const INDEX_UID: &str = "notes";

async fn url() -> Result<String, Box<dyn Error>> {
    let config = config()?;
    let protocol = if config.meili.ssl { "https" } else { "http" };
    Ok(format!("{}://{}:{}", protocol, config.meili.host, config.meili.port))
}

pub async fn connect_meili() -> Result<Client, Box<dyn Error>> {
    let config = config()?;
    let client = Client::new(&url().await?, config.meili.apikey.as_ref().map(String::as_str));
    let version = match client.get_version().await {
        Ok(version) => version,
        Err(e) => {
            println!("Error occurred while connecting to MeiliSearch: {}", e);
            return Err(e.into());
        }
    };
    println!("Connected to MeiliSearch: {}", version.pkg_version);
    Ok(client)
}

pub async fn reset(client: &Client) -> Result<(), Box<dyn Error>> {
    let config = config().unwrap();
    let http_client = reqwest::Client::new();
    let url = url().await.unwrap();

    if let Ok(task) = client.index(INDEX_UID).delete().await {
        println!("Delete: {}, {}", task.status, task.task_uid);
    } else {
        println!("Error occurred while deleting index. Skipping...");
    }

    let task = client.create_index(INDEX_UID, Some("id")).await.unwrap();
    println!("Create: {}, {}", task.status, task.task_uid);

    let settings = meilisearch_sdk::settings::Settings::new().with_searchable_attributes([
        "text",
        "cw",
    ]).with_sortable_attributes([
        "createdAt",
    ]).with_filterable_attributes([
        "createdAt",
        "userId",
        "userHost",
        "channelId",
    ]);

    let task: TaskInfo = client.index(INDEX_UID).set_settings(&settings).await.unwrap();
    println!("Settings: {}, {}", task.status, task.task_uid);

    let pagination = PaginationSetting { max_total_hits: 10000 };

    let task: TaskInfo = client.index(INDEX_UID).set_pagination(pagination).await.unwrap();
    println!("Pagination: {}, {}", task.status, task.task_uid);

    let request = json!({
        "enabled": false
    });

    let mut request_builder = http_client.patch(&format!("{}/indexes/{}/settings/typo-tolerance", url, INDEX_UID)).json(&request);

    if let Some(apikey) = &config.meili.apikey {
        request_builder = request_builder.header("Authorization", format!("Bearer {}", apikey));
    }

    let typo_tolerances = match request_builder.send().await {
        Ok(response) => response,
        Err(e) => {
            println!("Error occurred while setting typo tolerance: {}", e);
            return Err(e.into());
        }
    };
    let json = typo_tolerances.json::<TaskInfo>().await.unwrap();
    println!("Typo Tolerance: {}, {}", json.status, json.task_uid);

    Ok(())
}
