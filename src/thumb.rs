use std::fs::File;
use std::path::Path;

use actix_web::{web, HttpRequest, HttpResponse, Responder};

use crate::config::Config;
use crate::pathutils::{
    apply_cache_headers, check_file_cache, get_extension, get_real_path, is_supported_comic,
};
use crate::read::get_page_list_from_archive;

#[derive(serde::Deserialize)]
pub struct ThumbQuery {
    pub path: Option<String>,
}

/// Redirects a comic file request to the thumbnail image URL for the archive's first page.
///
/// If `query.path` is missing, responds with `400 BadRequest`. If the resolved filesystem
/// entry cannot be inspected or does not contain a first page, responds with `404 NotFound`.
/// If the resource is fresh according to cache checks, returns the cached `200 OK` response.
/// Otherwise returns a `301 Moved Permanently` response with `Location` set to
/// `/api/img?path=<url-encoded client path>&f=<url-encoded first page>&thumb=1` and applies cache headers for the target file.
///
/// # Examples
///
/// ```no_run
/// // Given a client path "comics/book.cbz" and first page "01.jpg",
/// // the handler will redirect to:
/// let client_path = "comics/book.cbz";
/// let first_page = "01.jpg";
/// let location = format!(
///     "/api/img?path={}&f={}&thumb=1",
///     urlencoding::encode(client_path),
///     urlencoding::encode(first_page),
/// );
/// assert!(location.contains("/api/img?path="));
/// ```
pub async fn thumb_handler(
    query: web::Query<ThumbQuery>,
    req: HttpRequest,
    config: web::Data<Config>,
) -> impl Responder {
    let client_path = match &query.path {
        Some(p) => p.clone(),
        None => return HttpResponse::BadRequest().finish(),
    };

    let real_path = match get_real_path(&client_path, &config) {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    let mut builder = HttpResponse::Ok();
    if check_file_cache(&real_path, &req, &mut builder) {
        return builder.finish();
    }

    let first_page = match get_first_page_name(&real_path) {
        Some(p) => p,
        None => return HttpResponse::NotFound().finish(),
    };

    let location = format!(
        "/api/img?path={}&f={}&thumb=1",
        urlencoding::encode(&client_path),
        urlencoding::encode(&first_page),
    );

    let mut resp = HttpResponse::MovedPermanently();
    apply_cache_headers(&real_path, &mut resp);
    resp.insert_header(("Location", location)).finish()
}

/// Handles directory thumbnail requests by locating the first supported comic in a directory and redirecting to its thumbnail URL.
///
/// If `query.path` is absent it defaults to `"/"`. The handler resolves the client path to a server filesystem path and may return an error response produced by the resolver. If a valid cached response is applicable, that cached response is returned. If no supported comic is found in the directory tree the handler returns `404 NotFound`. Otherwise it returns a `301 Moved Permanently` redirect to `/api/thumb?path=<encoded_path>` and applies cache headers for the target file.
///
/// # Returns
///
/// An `HttpResponse` that is either:
/// - a cached response (if the file cache indicates a valid cached result),
/// - the error response produced while resolving the client path,
/// - `404 NotFound` when no supported comic is found, or
/// - `301 Moved Permanently` redirecting to `/api/thumb?path=...` with appropriate cache headers.
///
/// # Examples
///
/// ```
/// # use actix_web::{test, web, HttpRequest};
/// # use crate::thumb::{dir_thumb_handler, ThumbQuery};
/// # async fn run_example() {
/// let query = web::Query(ThumbQuery { path: Some("/comics".into()) });
/// let req = test::TestRequest::default().to_http_request();
/// // `config` must be provided from your application; here represented as `config_data`.
/// // let resp = dir_thumb_handler(query, req, config_data).await;
/// # }
/// ```
pub async fn dir_thumb_handler(
    query: web::Query<ThumbQuery>,
    req: HttpRequest,
    config: web::Data<Config>,
) -> impl Responder {
    let client_path = query.path.clone().unwrap_or_else(|| "/".to_string());

    let real_path = match get_real_path(&client_path, &config) {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    let mut builder = HttpResponse::Ok();
    if check_file_cache(&real_path, &req, &mut builder) {
        return builder.finish();
    }

    // Walk the directory to find the first supported comic file.
    let thumb_path = find_first_comic_in_dir(&real_path, &client_path);

    if thumb_path == "" {
        return HttpResponse::NotFound().finish();
    }

    let location = format!("/api/thumb?path={}", urlencoding::encode(&thumb_path));

    let mut resp = HttpResponse::MovedPermanently();
    apply_cache_headers(&real_path, &mut resp);
    resp.insert_header(("Location", location)).finish()
}

/// Get the filename of the first image inside a CBZ archive.
///
/// If `comic_path` does not have a `cbz` extension, the archive cannot be opened,
/// the archive cannot be read, or the archive contains no image pages, this
/// function returns `None`.
///
/// # Returns
///
/// `Some(String)` containing the first image filename from the archive, `None` otherwise.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// // Non-existent or invalid archive yields None
/// assert!(get_first_page_name(Path::new("no-such-file.cbz")).is_none());
/// ```
#[tracing::instrument]
fn get_first_page_name(comic_path: &Path) -> Option<String> {
    let ext = get_extension(comic_path.to_str().unwrap_or(""));
    match ext.as_str() {
        "cbz" => {
            let file = File::open(comic_path).ok()?;
            let mut archive = zip::ZipArchive::new(file).ok()?;
            let pages = get_page_list_from_archive(&mut archive).ok()?;
            pages.into_iter().next().map(|p| p.image_file)
        }
        _ => None,
    }
}

/// Searches a directory (sorted using natural ordering) for the first supported comic file
/// and returns its path relative to `base_client_path`.
///
/// Returns an empty string if the directory cannot be read or if no supported comic file is found.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// // When the directory does not exist or contains no supported comics, an empty string is returned.
/// let found = find_first_comic_in_dir(Path::new("/nonexistent"), "/");
/// assert_eq!(found, "");
/// ```
#[tracing::instrument]
fn find_first_comic_in_dir(real_dir: &Path, base_client_path: &str) -> String {
    let entries = match std::fs::read_dir(real_dir) {
        Ok(e) => e,
        Err(_) => return String::new(),
    };

    for e in entries {
        if e.is_err() {
            continue;
        }
        let e = e.unwrap();
        let name = &e.file_name().to_string_lossy().to_string();
        let child = real_dir.join(&name);
        let child_client = format!("{}/{}", base_client_path.trim_end_matches('/'), name);

        if child.is_dir() {
            if let Some(found) = find_first_comic_recursive(&child, &child_client) {
                return found;
            }
        } else {
            let ext = get_extension(&name);
            if is_supported_comic(&ext) {
                return child_client;
            }
        }
    }

    String::new()
}

/// Searches a directory tree for the first supported comic file and returns its client-relative path.
///
/// This performs a natural-order sorted traversal of `real_dir`, recursing into subdirectories
/// and returning the first file whose extension is recognized as a supported comic.
///
/// # Parameters
///
/// - `real_dir`: Filesystem path to search.
/// - `client_prefix`: Client-visible path prefix corresponding to `real_dir` (no trailing `/` required).
///
/// # Returns
///
/// `Some(String)` containing the client-relative path to the first supported comic file found,
/// `None` if no supported comic is present or the directory cannot be read.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// // Assume a directory structure where "comic.cbz" exists under "tests/data"
/// let found = crate::thumb::find_first_comic_recursive(Path::new("tests/data"), "/base");
/// // `found` will be `Some("/base/comic.cbz")` if present, or `None` otherwise.
/// ```
#[tracing::instrument]
fn find_first_comic_recursive(real_dir: &Path, client_prefix: &str) -> Option<String> {
    let entries = std::fs::read_dir(real_dir).ok()?;

    for e in entries {
        if e.is_err() {
            continue;
        }
        let e = e.unwrap();
        let name = &e.file_name().to_string_lossy().to_string();
        let child = real_dir.join(&name);
        let child_client = format!("{}/{}", client_prefix.trim_end_matches('/'), name);

        if child.is_dir() {
            if let Some(found) = find_first_comic_recursive(&child, &child_client) {
                return Some(found);
            }
        } else {
            let ext = get_extension(&name);
            if is_supported_comic(&ext) {
                return Some(child_client);
            }
        }
    }
    None
}
