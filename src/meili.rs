use meilisearch_sdk::Client;
use meilisearch_sdk::settings::PaginationSetting;
use meilisearch_sdk::task_info::TaskInfo;
use serde_json::json;
use crate::config;

const INDEX_UID: &str = "notes";

pub async fn reset(client: &Client, url: &str, config: &config::Config) {
    let http_client = reqwest::Client::new();

    if let Ok(task) = client.index(INDEX_UID).delete().await {
        println!("Delete: {:?}", task);
    } else {
        println!("Error occurred while deleting index. Skipping...");
    }

    let task = client.create_index(INDEX_UID, Some("id")).await.unwrap();
    println!("Create: {:?}", task);

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
    println!("Settings: {:?}", task);

    let pagination = PaginationSetting { max_total_hits: 10000 };

    let task: TaskInfo = client.index(INDEX_UID).set_pagination(pagination).await.unwrap();
    println!("Pagination: {:?}", task);

    let request = json!({
        "enabled": false
    });

    let typo_tolerances = http_client.patch(&format!("{}/indexes/{}/settings/typo-tolerance", url, INDEX_UID)).json(&request).header("Authorization", format!("Bearer {}", config.meili.apikey)).send().await.unwrap();
    println!("Typo Tolerance: {:?}", typo_tolerances);
}