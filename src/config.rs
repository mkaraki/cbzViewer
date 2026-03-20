use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "cbzDir")]
    pub cbz_dir: String,
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string("config.json")?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}
