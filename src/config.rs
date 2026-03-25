use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "cbzDir")]
    pub cbz_dir: String,
}

/// Loads configuration from the local `config.json` file.
///
/// Reads `config.json` from the current working directory and parses it as JSON into a `Config`.
///
/// # Returns
///
/// `Ok(Config)` containing the parsed configuration on success, `Err` if reading the file or parsing fails.
///
/// # Examples
///
/// ```
/// let cfg = load_config().expect("failed to load config");
/// println!("cbz dir: {}", cfg.cbz_dir);
/// ```
pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string("config.json")?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}
