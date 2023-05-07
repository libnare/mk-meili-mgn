use std::error::Error;
use crate::config::config;
use tokio_postgres::{Client, NoTls};

pub async fn connect_db() -> Result<Client, Box<dyn Error>> {
    println!("Connecting to database");
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
            eprintln!("database connection error: {}", e);
        }
    });
    Ok(client)
}
