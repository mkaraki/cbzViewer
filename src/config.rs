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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_config_deserializes_cbz_dir() {
        let json = r#"{"cbzDir": "/srv/comics"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.cbz_dir, "/srv/comics");
    }

    #[test]
    fn test_config_cbz_dir_empty_string() {
        let json = r#"{"cbzDir": ""}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.cbz_dir, "");
    }

    #[test]
    fn test_config_cbz_dir_with_trailing_slash() {
        let json = r#"{"cbzDir": "/home/user/books/"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.cbz_dir, "/home/user/books/");
    }

    #[test]
    fn test_config_missing_cbz_dir_fails() {
        let json = r#"{"someOtherField": "value"}"#;
        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err(), "Expected deserialization to fail when cbzDir is missing");
    }

    #[test]
    fn test_config_invalid_json_fails() {
        let json = r#"not valid json"#;
        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_clone() {
        let json = r#"{"cbzDir": "/srv/comics"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        let cloned = config.clone();
        assert_eq!(cloned.cbz_dir, "/srv/comics");
    }

    #[test]
    fn test_load_config_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        let mut f = std::fs::File::create(&config_path).unwrap();
        write!(f, r#"{{"cbzDir": "/test/books"}}"#).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = load_config();
        std::env::set_current_dir(original_dir).unwrap();

        let config = result.expect("load_config should succeed with valid config.json");
        assert_eq!(config.cbz_dir, "/test/books");
    }

    #[test]
    fn test_load_config_missing_file_fails() {
        let dir = tempfile::tempdir().unwrap();
        // No config.json written in dir.
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = load_config();
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err(), "Expected error when config.json does not exist");
    }

    #[test]
    fn test_load_config_malformed_json_fails() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        let mut f = std::fs::File::create(&config_path).unwrap();
        write!(f, "{{ invalid json }}").unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = load_config();
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err(), "Expected error for malformed JSON");
    }
}