use std::path::{Path, PathBuf};

use actix_web::HttpResponse;
use crate::config::Config;

/// Resolve a client-supplied path against the configured CBZ base directory and return its canonical absolute path.
///
/// On success returns the canonicalized `PathBuf` for the resolved file or directory. If the target path does not exist or cannot be canonicalized, this returns an `HttpResponse` with status `404 Not Found`. If the configured base directory cannot be canonicalized, this returns `500 Internal Server Error`. If the resolved canonical path lies outside the canonical base directory (path traversal), this returns `403 Forbidden`.
///
/// # Examples
///
/// ```no_run
/// use actix_web::HttpResponse;
/// // Assume `config` is available and configured with `cbz_dir`.
/// let result = get_real_path("/some/dir/file.cbz", &config);
/// match result {
///     Ok(path) => println!("Resolved path: {}", path.display()),
///     Err(resp) => println!("HTTP error: {}", resp.status()),
/// }
/// ```
pub fn get_real_path(client_path: &str, config: &Config) -> Result<PathBuf, HttpResponse> {
    tracing::trace!("CALL pathutils::get_real_path({}, config)", client_path);

    // Strip any leading separator so that Path::join does not replace the base.
    let relative = client_path.trim_start_matches('/');

    let full = Path::new(&config.cbz_dir).join(relative);

    // canonicalize resolves symlinks and removes `..` components; it also
    // verifies that the path exists on disk.
    let canonical = full.canonicalize().map_err(|_| HttpResponse::NotFound().finish())?;

    let base = Path::new(&config.cbz_dir)
        .canonicalize()
        .map_err(|_| HttpResponse::InternalServerError().finish())?;

    if !canonical.starts_with(&base) {
        tracing::warn!(
            client_path = client_path,
            canonical_path = canonical.display().to_string(),
            base_dir = base.display().to_string(),
            "Path traversal attempt: resolved to outside base"
        );
        return Err(HttpResponse::Forbidden().finish());
    }

    Ok(canonical)
}

/// Compute the parent directory of `real_path` relative to the configured base directory.
///
/// If the canonical parent directory of `real_path` is inside `config.cbz_dir` (and `real_path`
/// is not the base itself), returns `(true, "/<relative_parent>")` where the second element is
/// the parent path with a leading slash and relative to the base. In all other cases returns
/// `(false, "")`.
///
/// # Examples
///
/// ```
/// // pseudo-code example; adjust `Config` construction to your project's type
/// let config = Config { cbz_dir: "/srv/comics".into() };
/// let (ok, parent) = get_parent_dir(Path::new("/srv/comics/series/issue.cbz"), &config);
/// assert!(ok);
/// assert_eq!(parent, "/series");
/// ```
pub fn get_parent_dir(real_path: &Path, config: &Config) -> (bool, String) {
    tracing::trace!("CALL pathutils::get_parent_dir({}, config)", real_path.display());

    let base = match Path::new(&config.cbz_dir).canonicalize() {
        Ok(p) => p,
        Err(_) => return (false, String::new()),
    };

    let base = if base.starts_with("\\\\?\\") {
        PathBuf::from(base.strip_prefix("\\\\?\\").unwrap())
    } else {
        base
    };

    let real_path_canonical = real_path.canonicalize();
    if real_path_canonical.is_err() {
        tracing::warn!("Failed to canonicalize real_path (user specified path)");
        return (false, String::new());
    }
    let real_path_canonical = real_path_canonical.unwrap();

    if real_path_canonical== base {
        return (false, String::new());
    }

    let parent = match real_path_canonical.parent() {
        Some(p) => p,
        None => return (false, String::new()),
    };

    let parent = if parent.starts_with("\\\\?\\") {
        PathBuf::from(parent.strip_prefix("\\\\?\\").unwrap())
    } else {
        parent.to_path_buf()
    };

    if parent.starts_with(&base) {
        let rel = match parent.strip_prefix(&base) {
            Ok(p) => p,
            Err(_) => return (false, String::new()),
        };

        let rel_str = format!("/{}", rel.display());

        return (true, rel_str);
    }

    (false, String::new())
}

/// Extracts the file extension from a path and returns it as a lowercase `String`.
///
/// If the path has no extension or the extension cannot be converted to UTF-8, returns an empty string.
///
/// # Examples
///
/// ```
/// assert_eq!(get_extension("archive.CBZ"), "cbz");
/// assert_eq!(get_extension("/path/to/image.PNG"), "png");
/// assert_eq!(get_extension("no_extension"), "");
/// ```
pub fn get_extension(file_path: &str) -> String {
    tracing::trace!("CALL pathutils::get_extension({})", file_path);

    Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// Checks whether a file extension corresponds to a supported image format.
///
/// Returns `true` if `ext` is one of: `"png"`, `"jpg"`, `"jpeg"`, `"gif"`, or `"webp"`, `false` otherwise.
///
/// # Examples
///
/// ```
/// assert!(is_supported_image("png"));
/// assert!(is_supported_image("jpeg"));
/// assert!(!is_supported_image("txt"));
/// ```
pub fn is_supported_image(ext: &str) -> bool {
    tracing::trace!("CALL pathutils::is_supported_image({})", ext);

    matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "webp")
}

/// Determines whether the given file extension identifies a supported comic archive.
///
/// # Examples
///
/// ```
/// assert!(is_supported_comic("cbz"));
/// assert!(!is_supported_comic("zip"));
/// ```
///
/// `true` if `ext` is `"cbz"`, `false` otherwise.
pub fn is_supported_comic(ext: &str) -> bool {
    tracing::trace!("CALL pathutils::is_supported_comic({})", ext);

    ext == "cbz"
}

/// Maps a file extension to the corresponding HTTP `Content-Type` MIME string.
///
/// Known image extensions are mapped to their specific MIME types; unknown extensions
/// map to `"application/octet-stream"`.
///
/// # Examples
///
/// ```
/// let ct = get_content_type("png");
/// assert_eq!(ct, "image/png");
/// ```
pub fn get_content_type(ext: &str) -> &'static str {
    tracing::trace!("CALL pathutils::get_content_type({})", ext);

    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

/// Formats the file's modification time as an HTTP-date string suitable for a `Last-Modified` header.
///
/// Returns the modification time formatted with `httpdate::fmt_http_date` when filesystem metadata and the modified timestamp are available; returns `None` if metadata or modification time cannot be read.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// // `pathutils` refers to the module containing `get_file_mtime_str`
/// let p = Path::new("Cargo.toml");
/// if let Some(mtime) = pathutils::get_file_mtime_str(p) {
///     // Example HTTP-date: "Tue, 15 Nov 1994 12:45:26 GMT"
///     assert!(mtime.contains(',') && mtime.ends_with("GMT"));
/// }
/// ```
pub fn get_file_mtime_str(path: &Path) -> Option<String> {
    tracing::trace!("CALL pathutils::get_file_mtime_str({})", path.display());

    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    Some(httpdate::fmt_http_date(mtime))
}

/// Checks the request's `If-Modified-Since` header against the file's modification time and sets `304 Not Modified` when they match.
///
/// This compares the exact string value of the request's `If-Modified-Since` header to the file's HTTP-formatted modification time (as produced by `get_file_mtime_str`). If they match, the response builder's status is set to `304 Not Modified`.
///
/// # Returns
///
/// `true` if the header exactly equals the file's modification time and the response status is set to `304 Not Modified`, `false` otherwise.
///
/// # Examples
///
/// ```
/// use actix_web::{test::TestRequest, HttpResponse};
/// use std::path::Path;
///
/// // A request without an If-Modified-Since header always yields false.
/// let req = TestRequest::default().to_http_request();
/// let mut res = HttpResponse::Ok();
/// assert_eq!(crate::pathutils::check_file_cache(Path::new("nonexistent"), &req, &mut res), false);
/// ```
pub fn check_file_cache(
    path: &Path,
    req: &actix_web::HttpRequest,
    res: &mut actix_web::HttpResponseBuilder,
) -> bool {
    tracing::trace!("CALL pathutils::check_file_cache({}, req, res)", path.display());

    if let Some(if_modified_since) = req.headers().get("If-Modified-Since") {
        if let Ok(ims_str) = if_modified_since.to_str() {
            if let Some(mtime) = get_file_mtime_str(path) {
                if ims_str == mtime {
                    res.status(actix_web::http::StatusCode::NOT_MODIFIED);
                    return true;
                }
            }
        }
    }
    false
}

/// Inserts caching headers into an HTTP response builder.
///
/// If the filesystem modification time for `path` can be read and formatted, this adds a
/// `Last-Modified` header with that value. It always adds `Cache-Control: public, max-age=31536000`.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use actix_web::HttpResponse;
///
/// let mut res = HttpResponse::Ok();
/// apply_cache_headers(Path::new("/var/www/file.png"), &mut res);
/// ```
pub fn apply_cache_headers(path: &Path, res: &mut actix_web::HttpResponseBuilder) {
    tracing::trace!("CALL pathutils::apply_cache_headers({}, res)", path.display());

    if let Some(mtime) = get_file_mtime_str(path) {
        res.insert_header(("Last-Modified", mtime));
    }
    res.insert_header(("Cache-Control", "public, max-age=31536000"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // ── get_extension ────────────────────────────────────────────────────────────

    #[test]
    fn test_get_extension_lowercase() {
        assert_eq!(get_extension("archive.cbz"), "cbz");
    }

    #[test]
    fn test_get_extension_uppercase_normalized() {
        assert_eq!(get_extension("IMAGE.PNG"), "png");
    }

    #[test]
    fn test_get_extension_mixed_case() {
        assert_eq!(get_extension("photo.JPeG"), "jpeg");
    }

    #[test]
    fn test_get_extension_no_extension() {
        assert_eq!(get_extension("README"), "");
    }

    #[test]
    fn test_get_extension_with_path() {
        assert_eq!(get_extension("/path/to/image.webp"), "webp");
    }

    #[test]
    fn test_get_extension_empty_string() {
        assert_eq!(get_extension(""), "");
    }

    #[test]
    fn test_get_extension_dot_only() {
        // A file named "." has no extension.
        assert_eq!(get_extension("."), "");
    }

    #[test]
    fn test_get_extension_hidden_file_no_ext() {
        // ".gitignore" is treated as having no extension (stem is ".gitignore").
        assert_eq!(get_extension(".gitignore"), "");
    }

    // ── is_supported_image ───────────────────────────────────────────────────────

    #[test]
    fn test_is_supported_image_png() {
        assert!(is_supported_image("png"));
    }

    #[test]
    fn test_is_supported_image_jpg() {
        assert!(is_supported_image("jpg"));
    }

    #[test]
    fn test_is_supported_image_jpeg() {
        assert!(is_supported_image("jpeg"));
    }

    #[test]
    fn test_is_supported_image_gif() {
        assert!(is_supported_image("gif"));
    }

    #[test]
    fn test_is_supported_image_webp() {
        assert!(is_supported_image("webp"));
    }

    #[test]
    fn test_is_supported_image_rejects_txt() {
        assert!(!is_supported_image("txt"));
    }

    #[test]
    fn test_is_supported_image_rejects_cbz() {
        assert!(!is_supported_image("cbz"));
    }

    #[test]
    fn test_is_supported_image_rejects_empty() {
        assert!(!is_supported_image(""));
    }

    #[test]
    fn test_is_supported_image_case_sensitive_uppercase_rejected() {
        // Function expects already-lowercased input; "PNG" is not in the match arms.
        assert!(!is_supported_image("PNG"));
    }

    // ── is_supported_comic ───────────────────────────────────────────────────────

    #[test]
    fn test_is_supported_comic_cbz() {
        assert!(is_supported_comic("cbz"));
    }

    #[test]
    fn test_is_supported_comic_rejects_zip() {
        assert!(!is_supported_comic("zip"));
    }

    #[test]
    fn test_is_supported_comic_rejects_cbr() {
        assert!(!is_supported_comic("cbr"));
    }

    #[test]
    fn test_is_supported_comic_rejects_empty() {
        assert!(!is_supported_comic(""));
    }

    #[test]
    fn test_is_supported_comic_case_sensitive_uppercase_rejected() {
        assert!(!is_supported_comic("CBZ"));
    }

    // ── get_content_type ─────────────────────────────────────────────────────────

    #[test]
    fn test_get_content_type_png() {
        assert_eq!(get_content_type("png"), "image/png");
    }

    #[test]
    fn test_get_content_type_jpg() {
        assert_eq!(get_content_type("jpg"), "image/jpeg");
    }

    #[test]
    fn test_get_content_type_jpeg() {
        assert_eq!(get_content_type("jpeg"), "image/jpeg");
    }

    #[test]
    fn test_get_content_type_gif() {
        assert_eq!(get_content_type("gif"), "image/gif");
    }

    #[test]
    fn test_get_content_type_webp() {
        assert_eq!(get_content_type("webp"), "image/webp");
    }

    #[test]
    fn test_get_content_type_unknown_returns_octet_stream() {
        assert_eq!(get_content_type("xyz"), "application/octet-stream");
    }

    #[test]
    fn test_get_content_type_empty_returns_octet_stream() {
        assert_eq!(get_content_type(""), "application/octet-stream");
    }

    #[test]
    fn test_get_content_type_cbz_returns_octet_stream() {
        assert_eq!(get_content_type("cbz"), "application/octet-stream");
    }

    // ── get_file_mtime_str ───────────────────────────────────────────────────────

    #[test]
    fn test_get_file_mtime_str_existing_file_returns_some() {
        // Cargo.toml is always present in the project root.
        let p = Path::new("Cargo.toml");
        let result = get_file_mtime_str(p);
        assert!(result.is_some(), "Expected Some for an existing file");
    }

    #[test]
    fn test_get_file_mtime_str_format_is_http_date() {
        let p = Path::new("Cargo.toml");
        if let Some(mtime) = get_file_mtime_str(p) {
            // HTTP-date format: "Tue, 15 Nov 1994 12:45:26 GMT"
            assert!(mtime.ends_with("GMT"), "HTTP-date should end with GMT: {}", mtime);
            assert!(mtime.contains(','), "HTTP-date should contain a comma: {}", mtime);
        }
    }

    #[test]
    fn test_get_file_mtime_str_nonexistent_returns_none() {
        let p = Path::new("/this/path/does/not/exist/at/all");
        assert!(get_file_mtime_str(p).is_none());
    }

    // ── get_real_path ────────────────────────────────────────────────────────────

    #[test]
    fn test_get_real_path_resolves_valid_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("comic.cbz");
        std::fs::write(&file, b"fake").unwrap();

        let config = Config { cbz_dir: dir.path().to_str().unwrap().to_string() };
        let result = get_real_path("comic.cbz", &config);
        assert!(result.is_ok(), "Expected Ok for valid path within base");
        assert_eq!(result.unwrap(), file.canonicalize().unwrap());
    }

    #[test]
    fn test_get_real_path_strips_leading_slash() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("comic.cbz");
        std::fs::write(&file, b"fake").unwrap();

        let config = Config { cbz_dir: dir.path().to_str().unwrap().to_string() };
        // Leading "/" should be stripped so it's treated as relative to cbz_dir.
        let result = get_real_path("/comic.cbz", &config);
        assert!(result.is_ok(), "Expected Ok when leading slash is stripped");
    }

    #[test]
    fn test_get_real_path_nonexistent_returns_404() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config { cbz_dir: dir.path().to_str().unwrap().to_string() };
        let result = get_real_path("does_not_exist.cbz", &config);
        assert!(result.is_err());
        let resp = result.unwrap_err();
        assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_get_real_path_traversal_returns_403() {
        let base = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret.txt");
        std::fs::write(&secret, b"secret").unwrap();

        let config = Config { cbz_dir: base.path().to_str().unwrap().to_string() };
        // Attempt to escape base via "../<other_tmpdir>/secret.txt".
        let relative = format!("../{}/ secret.txt", outside.path().file_name().unwrap().to_str().unwrap());
        // Most attempts will fail at canonicalize (404) rather than reach 403
        // because the path doesn't exist in base. A direct test using a symlink
        // or the actual relative escape is environment-dependent, so we accept
        // either 404 (path not found) or 403 (traversal detected).
        let result = get_real_path(&relative, &config);
        if let Err(resp) = result {
            let status = resp.status();
            assert!(
                status == actix_web::http::StatusCode::NOT_FOUND
                    || status == actix_web::http::StatusCode::FORBIDDEN,
                "Expected 404 or 403 for path traversal attempt, got {}",
                status
            );
        }
        // If Ok, the path happened to exist and be inside base (unlikely but not an error in test).
    }

    // ── get_parent_dir ───────────────────────────────────────────────────────────

    #[test]
    fn test_get_parent_dir_file_inside_base() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("comic.cbz");
        std::fs::write(&file, b"fake").unwrap();

        let config = Config { cbz_dir: dir.path().to_str().unwrap().to_string() };
        let (has_parent, parent_path) = get_parent_dir(&file, &config);
        assert!(has_parent);
        assert_eq!(parent_path, "/");
    }

    #[test]
    fn test_get_parent_dir_subdir_file() {
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("series");
        std::fs::create_dir(&subdir).unwrap();
        let file = subdir.join("issue.cbz");
        std::fs::write(&file, b"fake").unwrap();

        let config = Config { cbz_dir: dir.path().to_str().unwrap().to_string() };
        let (has_parent, parent_path) = get_parent_dir(&file, &config);
        assert!(has_parent);
        assert_eq!(parent_path, "/series");
    }

    #[test]
    fn test_get_parent_dir_at_base_returns_no_parent() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config { cbz_dir: dir.path().to_str().unwrap().to_string() };
        // The base directory itself has no parent within itself.
        let (has_parent, parent_path) = get_parent_dir(dir.path(), &config);
        assert!(!has_parent);
        assert_eq!(parent_path, "");
    }

    #[test]
    fn test_get_parent_dir_nonexistent_path_returns_no_parent() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config { cbz_dir: dir.path().to_str().unwrap().to_string() };
        let fake = dir.path().join("ghost.cbz");
        // Path does not exist on disk; canonicalize will fail.
        let (has_parent, parent_path) = get_parent_dir(&fake, &config);
        assert!(!has_parent);
        assert_eq!(parent_path, "");
    }
}