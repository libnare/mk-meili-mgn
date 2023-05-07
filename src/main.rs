mod database;
mod config;
mod r#struct;

use crate::config::config;
use crate::database::connect_db;
use crate::r#struct::Notes;
use serde_json::json;
use meilisearch_sdk::{client::*};
use meilisearch_sdk::settings::PaginationSetting;
use meilisearch_sdk::task_info::TaskInfo;
use reqwest;
use crossterm::{cursor::MoveToColumn, execute, style::{Color, Print, ResetColor, SetForegroundColor}, terminal::{Clear, ClearType}};
use std::{error::Error, io, sync::Mutex};

async fn reset(client: &Client, url: &str, config: &config::Config) {
    let http_client = reqwest::Client::new();

    let index_uid = "notes";

    if let Ok(task) = client.index(index_uid).delete().await {
        println!("Delete: {:?}", task);
    } else {
        println!("Error occurred while deleting index. Skipping...");
    }

    let task = client.create_index(index_uid, Some("id")).await.unwrap();
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

    let task: TaskInfo = client.index(index_uid).set_settings(&settings).await.unwrap();
    println!("Settings: {:?}", task);

    let pagination = PaginationSetting { max_total_hits: 10000 };

    let task: TaskInfo = client.index(index_uid).set_pagination(pagination).await.unwrap();
    println!("Pagination: {:?}", task);

    let request = json!({
        "enabled": false
    });

    let typo_tolerances = http_client.patch(&format!("{}/indexes/{}/settings/typo-tolerance", url, index_uid)).json(&request).header("Authorization", format!("Bearer {}", config.meili.apikey)).send().await.unwrap();
    println!("Typo Tolerance: {:?}", typo_tolerances);
}

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
