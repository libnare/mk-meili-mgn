mod database;
mod config;
mod r#struct;
mod reset;

use crate::config::config;
use crate::database::connect_db;
use crate::r#struct::Notes;
use serde_json::json;
use meilisearch_sdk::{client::*};
use crossterm::{cursor::MoveToColumn, execute, style::{Color, Print, ResetColor, SetForegroundColor}, terminal::{Clear, ClearType}};
use std::{error::Error, io, sync::Mutex};
use crate::reset::reset;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = config().unwrap();
    let db = connect_db().await.unwrap();

    let protocol = if config.meili.ssl { "https" } else { "http" };
    let url = format!("{}://{}:{}", protocol, config.meili.host, config.meili.port);
    println!("Connecting to MeiliSearch at {}", url);

    let client = Client::new(&url, Some(&config.meili.apikey));

    if config.meili.reset {
        reset(&client, &url, &config).await;
    } else {
        println!("Skipping reset");
    }

    let rows = db
        .query(
            "SELECT id, \"createdAt\", \"userId\", \"userHost\", \"channelId\", cw, text FROM note WHERE visibility NOT IN ('followers', 'specified') AND text IS NOT NULL ORDER BY \"createdAt\" DESC",
            &[],
        )
        .await?;
    let rows_len = rows.len();

    let mut stdout = io::stdout();

    let errors = Mutex::new(Vec::new());

    for (count, row) in rows.into_iter().enumerate() {
        let notes = Notes {
            id: row.get("id"),
            created_at: row.get("createdAt"),
            user_id: row.get("userId"),
            user_host: row.get("userHost"),
            channel_id: row.get("channelId"),
            cw: row.get("cw"),
            text: row.get("text"),
        };

        let timestamp = notes.created_at.timestamp_millis();
        let data = json!({
            "id": notes.id,
            "createdAt": timestamp,
            "userId": notes.user_id,
            "userHost": notes.user_host,
            "channelId": notes.channel_id,
            "cw": notes.cw,
            "text": notes.text,
        });

        let clear = Clear(ClearType::CurrentLine);
        let move_to_col = MoveToColumn(0);

        execute!(
            stdout,
            clear,
            move_to_col,
            SetForegroundColor(Color::Green),
            Print(format!("Count: {}, id: {}, createdAt: {}", count + 1, notes.id, notes.created_at)),
            ResetColor,
        )?;

        let index = client.index("notes");
        let res = index.add_documents(&[data], Some("id")).await;

        match res {
            Ok(res) => {
                execute!(
                    stdout,
                    clear,
                    move_to_col,
                    SetForegroundColor(Color::Cyan),
                    Print(format!("Add documents: {:?}\n", res.enqueued_at)),
                    ResetColor,
                )?;
            }
            Err(e) => {
                execute!(
                    stdout,
                    clear,
                    move_to_col,
                    SetForegroundColor(Color::Red),
                    Print(format!("add_documents error: {:?}", e)),
                    ResetColor,
                )?;
                errors.lock().unwrap().push(e);
            }
        }
    }

    let errors = errors.into_inner().unwrap();
    if !errors.is_empty() {
        println!("{} errors occurred", errors.len());
        for error in errors {
            let timestamp = chrono::Utc::now().timestamp_millis();
            std::fs::write(format!("error-{}.log", timestamp), format!("{:?}", error)).unwrap();
            println!("{:?}", error);
        }
    }

    println!("Done: {}", rows_len);

    Ok(())
}
