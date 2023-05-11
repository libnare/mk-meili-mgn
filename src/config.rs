use std::error::Error;
use std::fs::File;
use std::io::Read;
use serde::{Deserialize};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub db: DbConfig,
    pub meili: MeiliConfig,
}

#[derive(Debug, Deserialize)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Debug, Deserialize)]
pub struct MeiliConfig {
    pub host: String,
    pub port: u16,
    pub apikey: Option<String>,
    pub ssl: bool,
    pub reset: bool,
}


pub fn config() -> Result<Config, Box<dyn Error>> {
    let mut file = File::open("config.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: Config = serde_json::from_str(&contents)?;
    Ok(config)
}
