use std::error::Error;
use crate::config::config;
use tokio_postgres::{Client, NoTls};

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