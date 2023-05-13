mod database;
mod config;
mod r#struct;
mod meili;

use chrono::Utc;
use std::{error::Error, sync::Mutex};
use std::io::Write;
use kdam::{BarExt, Column, RichProgress, tqdm};
use kdam::term::Colorizer;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::{
    config::config,
    database::{connect_db, query_notes},
    meili::{connection, get_request_builder, reset, url, index_uid},
};

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
    let index_uid = index_uid().await.unwrap();
    let url = url().await.unwrap();

    connection().await.unwrap();

    if config.meili.reset {
        match reset().await {
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

    let mut pb = RichProgress::new(
        tqdm!(
            total = (data_len as u64).try_into().unwrap(),
            unit_divisor = 1024,
            unit = " Chunk"
        ),
        vec![
            Column::Spinner(
                "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
                    .chars()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>(),
                80.0,
                1.0,
            ),
            Column::text("[bold blue]?"),
            Column::Bar,
            Column::Percentage(1),
            Column::text("•"),
            Column::CountTotal,
            Column::text("•"),
            Column::Rate,
            Column::text("•"),
            Column::ElapsedTime,
        ],
    );

    let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    pb.write(&format!("<-- {}: Starting indexing. -->", time).colorize("yellow"));

    for (chunk_index, data_chunk) in data_chunks.enumerate() {
        pb.replace(1, Column::text("[bold blue]processing"));
        let json_array = match serde_json::to_string(data_chunk) {
            Ok(json_array) => json_array,
            Err(e) => {
                errors.lock().unwrap().push(format!("Error in chunk {}: {}", chunk_index, e));
                continue;
            }
        };

        let data = match serde_json::from_str(&json_array) {
            Ok(data) => data,
            Err(e) => {
                errors.lock().unwrap().push(format!("Error in chunk {}: {}", chunk_index, e));
                continue;
            }
        };

        let request_builder = get_request_builder(
            &url,
            format!("indexes/{}/documents", index_uid).as_str(),
            data,
            reqwest::Method::POST,
        ).await.unwrap();

        let res = request_builder.send().await?;
        let res_status = res.status();

        if res_status.is_success() {
            total_added += data_chunk.len();
            let new = std::cmp::min(total_added + chunk_size, data_len);
            pb.update_to(new);
        } else {
            errors.lock().unwrap().push(
                format!("Error in chunk {}: {}",
                        chunk_index, res.text().await.unwrap()
                ));
        }
    }
    pb.replace(1, Column::text("[bold blue]done"));

    let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    pb.write(&format!("<-- {}: Finished indexing. -->", time).colorize("yellow"));

    let errors = errors.into_inner().unwrap();
    let total_skipped = errors.len();

    if total_skipped > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        writeln!(&mut stdout, "\n{} errors occurred", total_skipped)?;
        let timestamp = Utc::now().timestamp_millis();
        std::fs::write(format!("error-{}.log", timestamp), errors.join("\n")).unwrap();
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        writeln!(&mut stdout, "\nAll errors have been output to error-{}.log", timestamp)?;
        stdout.reset()?;
    }

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
    writeln!(&mut stdout, "\n{} notes have been added", total_added)?;
    stdout.reset()?;
    if total_skipped > 0 {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(0x00, 0x8b, 0x8b))))?; // Dark cyan
        writeln!(&mut stdout, "\n{} notes were skipped due to errors", total_skipped)?;
        stdout.reset()?;
    }

    Ok(())
}
