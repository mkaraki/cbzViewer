use std::path::Path;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::Serialize;

use crate::config::Config;
use crate::pathutils::{apply_cache_headers, check_file_cache, get_extension, get_parent_dir, get_real_path, is_supported_comic};

#[derive(Serialize)]
pub struct ListItem {
    pub name: String,
    pub path: String,
    #[serde(rename = "isDir")]
    pub is_dir: bool,
}

#[derive(Serialize)]
pub struct ListData {
    pub items: Vec<ListItem>,
    #[serde(rename = "currentDir")]
    pub current_dir: String,
    #[serde(rename = "hasParent")]
    pub has_parent: bool,
    #[serde(rename = "parentDir")]
    pub parent_dir: String,
}

#[derive(serde::Deserialize)]
pub struct ListQuery {
    pub path: Option<String>,
}

pub async fn list_handler(
    query: web::Query<ListQuery>,
    req: HttpRequest,
    config: web::Data<Config>,
) -> impl Responder {
    let client_path = query.path.clone().unwrap_or_else(|| "/".to_string());

    let real_path = match get_real_path(&client_path, &config) {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    // Check HTTP cache before doing any work.
    let mut builder = HttpResponse::Ok();
    if check_file_cache(&real_path, &req, &mut builder) {
        return builder.finish();
    }

    let entries = match std::fs::read_dir(&real_path) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            sentry::capture_error(&e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let (has_parent, parent_dir) = get_parent_dir(&real_path, &config);

    let mut items: Vec<ListItem> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let ext = get_extension(&name);

        if !is_dir && !is_supported_comic(&ext) {
            continue;
        }

        let item_path = Path::new(&client_path).join(&name);
        let path_str = format!("/{}", item_path.to_string_lossy().trim_start_matches('/'));

        items.push(ListItem {
            name,
            path: path_str,
            is_dir,
        });
    }

    items.sort_by(|a, b| natord::compare(&a.name, &b.name));

    let list_data = ListData {
        items,
        current_dir: client_path,
        has_parent,
        parent_dir,
    };

    apply_cache_headers(&real_path, &mut builder);
    builder
        .content_type("application/json")
        .json(list_data)
}
