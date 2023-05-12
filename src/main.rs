mod database;
mod config;
mod r#struct;
mod meili;

const INDEX_UID: &str = "notes";

use crate::config::config;
use crate::database::connect_db;
use crate::r#struct::Notes;
use crossterm::{cursor::MoveToColumn, execute, style::{Color, Print, ResetColor, SetForegroundColor}, terminal::{Clear, ClearType}};
use std::{error::Error, io, sync::Mutex};
use chrono::{DateTime, Utc};
use crate::meili::{connect_meili, get_request_builder, reset, url};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = config().unwrap_or_else(|err| {
        println!("Configuration error: {}", err);
        std::process::exit(1);
    });
    let db = connect_db().await.unwrap();
    let client = connect_meili().await.unwrap();

    if config.meili.reset {
        reset(&client).await.expect("Failed to MeiliSearch reset")
    } else {
        println!("Skipping reset");
    }

    let rows = db
        .query("
        SELECT id, \"createdAt\", \"userId\", \"userHost\", \"channelId\", cw, text
        FROM note
        WHERE COALESCE(text, cw) IS NOT NULL
          AND visibility IN ('home', 'public')
          AND text IS NOT NULL",
            &[],
        )
        .await?;
    let rows_len = rows.len();

    let mut data_vec = Vec::new();

    for row in rows {
        let created_at: DateTime<Utc> = row.get("createdAt");
        let notes = Notes {
            id: row.get("id"),
            created_at: created_at.timestamp() * 1000 + created_at.timestamp_subsec_millis() as i64,
            user_id: row.get("userId"),
            user_host: row.get("userHost"),
            channel_id: row.get("channelId"),
            cw: row.get("cw"),
            text: row.get("text"),
        };

        data_vec.push(notes);
    }

    let mut stdout = io::stdout();
    let errors = Mutex::new(Vec::new());

    let chunk_size = 19456;
    let data_chunks = data_vec.chunks(chunk_size);

    for (chunk_index, data_chunk) in data_chunks.enumerate() {
        let json_array = serde_json::to_string(data_chunk).unwrap();
        let clear = Clear(ClearType::CurrentLine);
        let move_to_col = MoveToColumn(0);

        let http_client = reqwest::Client::new();
        let url = url().await.unwrap();

        let request_builder = get_request_builder(
            &http_client,
            &url,
            INDEX_UID,
            "documents",
            serde_json::from_str(&json_array).unwrap(),
            reqwest::Method::POST,
        ).await.unwrap();

        let res = match request_builder.send().await {
            Ok(res) => res,
            Err(e) => {
                execute!(
            stdout,
            clear,
            move_to_col,
            SetForegroundColor(Color::Red),
            Print(format!("Add documents error: {:?}", e)),
            ResetColor,
        )?;
                errors.lock().unwrap().push(e);
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
        std::fs::write(format!("error-{}.log", timestamp), format!("{:?}", errors)).unwrap();
        println!("All errors have been output to error-{}.log", timestamp);
    }

    execute!(
        stdout,
        Clear(ClearType::CurrentLine),
        MoveToColumn(0),
        SetForegroundColor(Color::Green),
        Print(format!("{} notes have been added\n", rows_len - errors.len())),
        ResetColor,
    )?;

    Ok(())
}
