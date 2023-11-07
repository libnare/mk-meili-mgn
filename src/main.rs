use std::{error::Error, sync::Mutex};

use chrono::Utc;
use serde_json::json;
use tokio::signal;

use console::Style;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    config::config,
    database::{connect_db, query_notes},
    meili::{connection, index_uid, reset, url},
};
use crate::client::Client;

mod database;
mod config;
mod r#struct;
mod meili;
mod aid_series;
mod client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let config = match config() {
        Ok(config) => config,
        Err(err) => {
            let red = Style::new().color256(196);
            println!("{}: {}", red.apply_to("Configuration error"), err);
            std::process::exit(1);
        }
    };

    let sigint = signal::ctrl_c();
    let db = connect_db().await?;
    let index_uid = index_uid();
    let client = Client::new(config.meili.apikey)?;

    connection(&client).await?;

    if config.meili.reset {
        match reset(&client).await {
            Ok(_) => {
                let green = Style::new().color256(28);
                println!("{}", green.apply_to("Meilisearch index reset."));
            }
            Err(e) => {
                let red = Style::new().color256(196);
                println!("{}: {}", red.apply_to("Failed to reset Meilisearch index"), e);
                std::process::exit(1);
            }
        }
    }

    let data_vec = query_notes(&db).await.unwrap();
    let data_len = data_vec.len();

    let sky_blue = Style::new().color256(111);
    println!("Retrieved {} notes from database.", sky_blue.apply_to(data_len));

    let errors = Mutex::new(Vec::new());
    let chunk_size = 19456; // https://stella.place/notes/9eo7ew8sed
    let data_chunks = data_vec.chunks(chunk_size);
    let mut total_added = 0;

    let pb = ProgressBar::new(data_len as u64);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
        .unwrap()
        .progress_chars("#>-")
    );

    tokio::select! {
        _ = sigint => {
            let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let red = Style::new().color256(160);
            pb.finish();
            println!("<-- {}: Indexing interrupted. -->", red.apply_to(time));
            println!("{}", red.apply_to("Program terminated by user."));
            std::process::exit(1);
        }
        _ = async {
            pb.set_prefix("processing");
            let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let yellow = Style::new().color256(154);
            pb.println(&format!("<-- {}: Starting indexing. -->", yellow.apply_to(time)));

            for (chunk_index, data_chunk) in data_chunks.enumerate() {
                let data = json!(data_chunk);

                let res = match client
                    .post(format!("{}/{}", url(), format!("indexes/{}/documents", index_uid).as_str()))
                    .json(&data)
                    .send().await {
                    Ok(res) => res,
                    Err(e) => {
                        errors.lock().unwrap().push(
                            format!("Error in chunk {}: {}", chunk_index, e)
                        );
                        continue;
                    }
                };

                if res.status().is_success() {
                    total_added += data_chunk.len();
                    let new = std::cmp::min(total_added + chunk_size, data_len) as u64;
                    pb.set_position(new);
                } else {
                    errors.lock().unwrap().push(
                        format!("Error in chunk {}: {}",
                            chunk_index, res.text().await.unwrap()
                        )
                    );
                }
            }
            pb.finish_with_message("done");
            let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let green = Style::new().color256(69);
            pb.println(&format!("<-- {}: Indexing completed. -->", green.apply_to(time)));
        } => {}
    }

    let errors = errors.into_inner().unwrap();
    let total_skipped = errors.len();

    if total_skipped > 0 {
        let red = Style::new().color256(196);
        println!("\n{} errors occurred", red.apply_to(total_skipped));
        let timestamp = Utc::now().timestamp_millis();
        std::fs::write(format!("error-{}.log", timestamp), errors.join("\n")).unwrap();
        let yellow = Style::new().color256(226);
        println!("{}", yellow.apply_to(format!("\nAll errors have been output to error-{}.log", timestamp)));
    }

    let sky_blue = Style::new().color256(111);
    println!("{} notes have been added", sky_blue.apply_to(total_added));
    if total_skipped > 0 {
        let cyan = Style::new().color256(37);
        println!("\n{} notes were skipped due to errors", cyan.apply_to(total_skipped));
    }

    Ok(())
}
