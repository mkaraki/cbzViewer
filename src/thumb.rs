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

    let location = format!("/api/thumb?path={}", urlencoding::encode(&thumb_path));

    let mut resp = HttpResponse::MovedPermanently();
    apply_cache_headers(&real_path, &mut resp);
    resp.insert_header(("Location", location)).finish()
}

/// Returns the name of the first image inside the CBZ archive.
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

/// Walks the directory tree to locate the first comic file, returning its
/// path relative to `base_client_path`.
fn find_first_comic_in_dir(real_dir: &Path, base_client_path: &str) -> String {
    let entries = match std::fs::read_dir(real_dir) {
        Ok(e) => e,
        Err(_) => return String::new(),
    };

    let mut names: Vec<_> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort_by(|a, b| natord::compare(a, b));

    for name in names {
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

fn find_first_comic_recursive(real_dir: &Path, client_prefix: &str) -> Option<String> {
    let entries = std::fs::read_dir(real_dir).ok()?;
    let mut names: Vec<_> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort_by(|a, b| natord::compare(a, b));

    for name in names {
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
