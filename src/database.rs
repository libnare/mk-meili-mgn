use std::error::Error;

use chrono::{DateTime, Utc};
use tokio_postgres::{Client, NoTls};

use crate::config::config;
use crate::r#struct::Notes;
use crate::aid_series;

pub async fn connect_db() -> Result<Client, Box<dyn Error>> {
    let config = config()?;
    let (client, connection) = tokio_postgres::connect(
        &format!(
            "host={} port={} user={} password={} dbname={}",
            config.db.host, config.db.port, config.db.user, config.db.password, config.db.database
        ),
        NoTls,
    ).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Error occurred while connecting to database: {}", e);
        }
    });

    let row = client.query_one("SELECT version()", &[]).await?;
    let version: &str = row.try_get(0)?;

    println!("Connected to PostgreSQL: {}", version);

    Ok(client)
}

pub async fn query_notes(db: &Client) -> Result<Vec<Notes>, Box<dyn Error>> {
    let config = config()?;

    let mut query = std::string::String::from("
        SELECT id, ");

    if config.option.idtype.is_none() {
        query.push_str("\"createdAt\", ");
    }

    query.push_str("\"userId\", \"userHost\", \"channelId\", cw, text, tags
        FROM note
        WHERE COALESCE(text, cw) IS NOT NULL
          AND visibility IN ('home', 'public')
          AND text IS NOT NULL");

    if config.option.localonly {
        query.push_str(" AND \"userHost\" IS NULL");
    }

    if let Some(limit) = config.option.limit {
        query.push_str(&format!(" LIMIT {}", limit));
    }

    let rows = db.query(&query, &[]).await?;

    let mut data_vec = Vec::new();

    if config.option.idtype.is_none() {
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
                tags: row.get("tags"),
            };

            data_vec.push(notes);
        }
    } else if let Some(idtype) = config.option.idtype.as_ref() {
        if idtype == "aid" || idtype == "aidx" {
            for row in rows {
                let created_at: DateTime<Utc> = aid_series::parse(row.get("id"));
                let notes = Notes {
                    id: row.get("id"),
                    created_at: created_at.timestamp() * 1000 + created_at.timestamp_subsec_millis() as i64,
                    user_id: row.get("userId"),
                    user_host: row.get("userHost"),
                    channel_id: row.get("channelId"),
                    cw: row.get("cw"),
                    text: row.get("text"),
                    tags: row.get("tags"),
                };

                data_vec.push(notes);
            }
        } else {
            panic!("Invalid idtype: {}", idtype);
        }
    }

    Ok(data_vec)
}
