mod database;
mod config;
mod r#struct;
mod meili;

use std::{error::Error, sync::Mutex};
use chrono::Utc;

use crate::{
    config::config,
    database::connect_db,
    meili::{connect_meili, get_request_builder, reset, url},
};
use crate::database::query_notes;

const INDEX_UID: &str = "notes";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = config().unwrap_or_else(|err| {
        println!("Configuration error: {}", err);
        std::process::exit(1);
    });
    let db = connect_db().await.unwrap();
    let client = connect_meili().await.unwrap();

    if config.meili.reset {
        reset(&client).await.expect("Failed to Meilisearch reset")
    } else {
        println!("Skipping reset");
    }

    let data_vec = query_notes(&db).await.unwrap();
    let data_len = data_vec.len();

    let errors = Mutex::new(Vec::new());

    let chunk_size = 19456; // https://stella.place/notes/9eo7ew8sed
    let data_chunks = data_vec.chunks(chunk_size);

    for (chunk_index, data_chunk) in data_chunks.enumerate() {
        let json_array = match serde_json::to_string(data_chunk) {
            Ok(json_array) => json_array,
            Err(e) => {
                errors.lock().unwrap().push(format!("{}: error: {:?}", chunk_index, e));
                continue;
            }
        };

        let http_client = reqwest::Client::new();
        let url = url().await.unwrap();

        let data = match serde_json::from_str(&json_array) {
            Ok(data) => data,
            Err(e) => {
                errors.lock().unwrap().push(format!("{}: error: {:?}", chunk_index, e));
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
                errors.lock().unwrap().push(format!("{}: error: {:?}", chunk_index, e));
                continue;
            }
        };

        let res_status = res.status();
        let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        println!("Add documents: {}, {}", res_status, time);
    }

    let errors = errors.into_inner().unwrap();
    if !errors.is_empty() {
        println!("{} errors occurred", errors.len());
        let timestamp = Utc::now().timestamp_millis();
        std::fs::write(format!("error-{}.log", timestamp), errors.join("\n")).unwrap();
        println!("All errors have been output to error-{}.log", timestamp);
    }

    println!("{} notes have been added", data_len - errors.len());

    Ok(())
}
