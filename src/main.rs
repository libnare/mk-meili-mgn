mod database;
mod config;
mod r#struct;
mod meili;

use chrono::Utc;
use std::{error::Error, sync::Mutex};

use crate::{
    config::config,
    database::{connect_db, query_notes},
    meili::{connect_meili, get_request_builder, reset, url},
};

const INDEX_UID: &str = "notes";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = config().unwrap_or_else(|err| {
        eprintln!("Configuration error: {}", err);
        std::process::exit(1);
    });

    let db = connect_db().await.unwrap();

    let client = connect_meili().await.unwrap();

    if config.meili.reset {
        reset(&client).await.expect("Failed to reset Meilisearch index");
        println!("Meilisearch index reset.");
    }

    let data_vec = query_notes(&db).await.unwrap();
    let data_len = data_vec.len();
    println!("Retrieved {} notes from database.", data_len);

    let errors = Mutex::new(Vec::new());
    let chunk_size = 19456; // https://stella.place/notes/9eo7ew8sed
    let data_chunks = data_vec.chunks(chunk_size);
    let mut total_added = 0;

    for (chunk_index, data_chunk) in data_chunks.enumerate() {
        let json_array = match serde_json::to_string(data_chunk) {
            Ok(json_array) => json_array,
            Err(e) => {
                errors.lock().unwrap().push(format!("Error in chunk {}: {}", chunk_index, e));
                continue;
            }
        };

        let http_client = reqwest::Client::new();
        let url = url().await.unwrap();

        let data = match serde_json::from_str(&json_array) {
            Ok(data) => data,
            Err(e) => {
                errors.lock().unwrap().push(format!("Error in chunk {}: {}", chunk_index, e));
                continue;
            }
        };

        let request_builder = get_request_builder(
            &http_client,
            &url,
            INDEX_UID,
            "documents",
            data,
            reqwest::Method::POST,
        ).await.unwrap();

        let res = match request_builder.send().await {
            Ok(res) => res,
            Err(e) => {
                errors.lock().unwrap().push(format!("Error in chunk {}: {}", chunk_index, e));
                continue;
            }
        };

        let res_status = res.status();
        let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        println!("Added documents from chunk {}: {}, {}", chunk_index, res_status, time);

        if res_status.is_success() {
            total_added += data_chunk.len();
        }
    }

    let errors = errors.into_inner().unwrap();
    let total_skipped = errors.len();

    if total_skipped > 0 {
        println!("{} errors occurred", total_skipped);
        let timestamp = Utc::now().timestamp_millis();
        std::fs::write(format!("error-{}.log", timestamp), errors.join("\n")).unwrap();
        println!("All errors have been output to error-{}.log", timestamp);
    }

    println!("{} notes have been added", total_added);
    if total_skipped > 0 {
        println!("{} notes were skipped due to errors", total_skipped);
    }

    Ok(())
}
