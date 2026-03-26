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

    // Guard to serialise tests that touch the working directory.
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_config_json<F: FnOnce()>(content: &str, f: F) {
        let _guard = CWD_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.json");
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        drop(file);

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        f();
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_config_deserializes_cbz_dir() {
        with_config_json(r#"{"cbzDir": "/srv/comics"}"#, || {
            let cfg = load_config().expect("load_config should succeed");
            assert_eq!(cfg.cbz_dir, "/srv/comics");
        });
    }

    #[test]
    fn test_config_clone() {
        with_config_json(r#"{"cbzDir": "/books"}"#, || {
            let cfg = load_config().unwrap();
            let cloned = cfg.clone();
            assert_eq!(cloned.cbz_dir, cfg.cbz_dir);
        });
    }

    #[test]
    fn test_load_config_missing_file_returns_error() {
        let _guard = CWD_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        // Do NOT create config.json
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        let result = load_config();
        std::env::set_current_dir(original_dir).unwrap();
        assert!(result.is_err(), "expected error when config.json is missing");
    }

    #[test]
    fn test_load_config_invalid_json_returns_error() {
        with_config_json("not valid json }{", || {
            let result = load_config();
            assert!(result.is_err(), "expected error for invalid JSON");
        });
    }

    #[test]
    fn test_load_config_missing_field_returns_error() {
        // JSON object without required "cbzDir" key
        with_config_json(r#"{"someOtherField": "value"}"#, || {
            let result = load_config();
            assert!(result.is_err(), "expected error when cbzDir field is absent");
        });
    }

    #[test]
    fn test_load_config_empty_cbz_dir() {
        with_config_json(r#"{"cbzDir": ""}"#, || {
            let cfg = load_config().expect("should parse empty string");
            assert_eq!(cfg.cbz_dir, "");
        });
    }

    #[test]
    fn test_load_config_cbz_dir_with_trailing_slash() {
        with_config_json(r#"{"cbzDir": "/srv/comics/"}"#, || {
            let cfg = load_config().expect("should parse path with trailing slash");
            assert_eq!(cfg.cbz_dir, "/srv/comics/");
        });
    }
}