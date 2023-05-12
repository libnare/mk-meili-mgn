mod database;
mod config;
mod r#struct;
mod meili;

use chrono::Utc;
use std::{error::Error, sync::Mutex};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::{
    config::config,
    database::{connect_db, query_notes},
    meili::{connect_meili, get_request_builder, reset, url},
};

const INDEX_UID: &str = "notes";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    let config = match config() {
        Ok(config) => config,
        Err(err) => {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
            writeln!(&mut stdout, "Configuration error: {}", err)?;
            stdout.reset()?;
            std::process::exit(1);
        }
    };

    let db = connect_db().await.unwrap();

    let client = connect_meili().await.unwrap();

    if config.meili.reset {
        match reset(&client).await {
            Ok(_) => {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                writeln!(&mut stdout, "Meilisearch index reset.")?;
                stdout.reset()?;
            }
            Err(e) => {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                writeln!(&mut stdout, "Failed to reset Meilisearch index: {}", e)?;
                stdout.reset()?;
                std::process::exit(1);
            }
        }
    }

    let data_vec = query_notes(&db).await.unwrap();
    let data_len = data_vec.len();
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(0x87, 0xce, 0xeb))))?; // Sky blue
    writeln!(&mut stdout, "Retrieved {} notes from database.", data_len)?;
    stdout.reset()?;

    let errors = Mutex::new(Vec::new());
    let chunk_size = 19456; // https://stella.place/notes/9eo7ew8sed
    let data_chunks = data_vec.chunks(chunk_size);
    let mut total_added = 0;

    let pb = ProgressBar::new(data_len as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)").unwrap()
            .progress_chars("#>-"),
    );

    let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    writeln!(&mut stdout, "<-- {}: Starting indexing. -->", time)?;
    stdout.reset()?;

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

        if res_status.is_success() {
            total_added += data_chunk.len();
            pb.inc(data_chunk.len() as u64);
        } else {
            errors.lock().unwrap().push(
                format!("Error in chunk {}: {}",
                        chunk_index, res.text().await.unwrap()
                ));
        }
    }

    pb.finish();

    let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    writeln!(&mut stdout, "<-- {}: Finished indexing. -->", time)?;
    stdout.reset()?;

    let errors = errors.into_inner().unwrap();
    let total_skipped = errors.len();

    if total_skipped > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        writeln!(&mut stdout, "{} errors occurred", total_skipped)?;
        let timestamp = Utc::now().timestamp_millis();
        std::fs::write(format!("error-{}.log", timestamp), errors.join("\n")).unwrap();
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        writeln!(&mut stdout, "All errors have been output to error-{}.log", timestamp)?;
        stdout.reset()?;
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
    writeln!(&mut stdout, "{} notes have been added", total_added)?;
    stdout.reset()?;
    if total_skipped > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(0x00, 0x8b, 0x8b))))?; // Dark cyan
        writeln!(&mut stdout, "{} notes were skipped due to errors", total_skipped)?;
        stdout.reset()?;
    }

    Ok(())
}
