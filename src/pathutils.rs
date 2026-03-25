use std::path::{Path, PathBuf};

use actix_web::HttpResponse;
use log::trace;
use crate::config::Config;

/// Resolves and validates a client-supplied relative path against the configured
/// base directory.  Returns the canonicalised absolute `PathBuf` or an
/// appropriate `HttpResponse` error.
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

/// Returns `(has_parent, parent_dir_relative_to_base)`.
pub fn get_parent_dir(real_path: &Path, config: &Config) -> (bool, String) {
    tracing::trace!("CALL pathutils::get_parent_dir({}, config)", real_path.display());

    let base = match Path::new(&config.cbz_dir).canonicalize() {
        Ok(p) => p,
        Err(_) => return (false, String::new()),
    };

    let real_path_canonical = Path::new(real_path).canonicalize();
    if real_path_canonical.is_err() {
        tracing::warn!("Failed to canonicalize real_path (user specified path)");
        return (false, String::new());
    }

    if real_path.canonicalize().unwrap() == base {
        return (false, String::new());
    }

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

        return (true, rel_str);
    }

    (false, String::new())
}

/// Returns the lowercase file extension, or an empty string if none.
pub fn get_extension(file_path: &str) -> String {
    tracing::trace!("CALL pathutils::get_extension({})", file_path);

    Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase()
}

pub fn is_supported_image(ext: &str) -> bool {
    tracing::trace!("CALL pathutils::is_supported_image({})", ext);

    matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "webp")
}

pub fn is_supported_comic(ext: &str) -> bool {
    tracing::trace!("CALL pathutils::is_supported_comic({})", ext);

    ext == "cbz"
}

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

/// Returns the HTTP-formatted `Last-Modified` value for `path`, if available.
pub fn get_file_mtime_str(path: &Path) -> Option<String> {
    tracing::trace!("CALL pathutils::get_file_mtime_str({})", path.display());

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

/// Adds `Last-Modified` and `Cache-Control` headers to a response builder.
pub fn apply_cache_headers(path: &Path, res: &mut actix_web::HttpResponseBuilder) {
    tracing::trace!("CALL pathutils::apply_cache_headers({}, res)", path.display());

    if let Some(mtime) = get_file_mtime_str(path) {
        res.insert_header(("Last-Modified", mtime));
    }
    res.insert_header(("Cache-Control", "public, max-age=31536000"));
}
