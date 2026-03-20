use std::path::{Path, PathBuf};

use actix_web::HttpResponse;

use crate::config::Config;

/// Resolves and validates a client-supplied relative path against the configured
/// base directory.  Returns the canonicalised absolute `PathBuf` or an
/// appropriate `HttpResponse` error.
pub fn get_real_path(client_path: &str, config: &Config) -> Result<PathBuf, HttpResponse> {
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
        log::warn!(
            "Path traversal attempt: {} resolved to {} outside base {}",
            client_path,
            canonical.display(),
            base.display()
        );
        return Err(HttpResponse::Forbidden().finish());
    }

    Ok(canonical)
}

/// Returns `(has_parent, parent_dir_relative_to_base)`.
pub fn get_parent_dir(real_path: &Path, config: &Config) -> (bool, String) {
    let base = match Path::new(&config.cbz_dir).canonicalize() {
        Ok(p) => p,
        Err(_) => return (false, String::new()),
    };

    let parent = match real_path.parent() {
        Some(p) => p,
        None => return (false, String::new()),
    };

    if parent.starts_with(&base) {
        let rel = match parent.strip_prefix(&base) {
            Ok(p) => p,
            Err(_) => return (false, String::new()),
        };

        let rel_str = format!("/{}", rel.display());

        if rel_str == "/" {
            return (false, String::new());
        }

        return (true, rel_str);
    }

    (false, String::new())
}

/// Returns the lowercase file extension, or an empty string if none.
pub fn get_extension(file_path: &str) -> String {
    file_path
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

pub fn is_supported_image(ext: &str) -> bool {
    matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif")
}

pub fn is_supported_comic(ext: &str) -> bool {
    ext == "cbz"
}

pub fn get_content_type(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    }
}

/// Returns the HTTP-formatted `Last-Modified` value for `path`, if available.
pub fn get_file_mtime_str(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    Some(httpdate::fmt_http_date(mtime))
}

/// Returns `true` and writes `304 Not Modified` when the client's cached copy
/// is still current.  Returns `false` when the client needs a fresh response.
pub fn check_file_cache(
    path: &Path,
    req: &actix_web::HttpRequest,
    res: &mut actix_web::HttpResponseBuilder,
) -> bool {
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

/// Adds `Last-Modified` and `Cache-Control` headers to a response builder.
pub fn apply_cache_headers(path: &Path, res: &mut actix_web::HttpResponseBuilder) {
    if let Some(mtime) = get_file_mtime_str(path) {
        res.insert_header(("Last-Modified", mtime));
    }
    res.insert_header(("Cache-Control", "public, max-age=31536000"));
}
