use std::fs::File;
use std::io::Read;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::pathutils::{
    apply_cache_headers, check_file_cache, get_extension, get_parent_dir, get_real_path,
    is_supported_image,
};

#[derive(Serialize)]
pub struct PageInfo {
    #[serde(rename = "pageNo")]
    pub page_no: usize,
    #[serde(rename = "imageFile")]
    pub image_file: String,
}

#[derive(Serialize)]
pub struct ReadInfo {
    #[serde(rename = "comicTitle")]
    pub comic_title: String,
    pub pages: Vec<PageInfo>,
    pub path: String,
    #[serde(rename = "pageCnt")]
    pub page_cnt: usize,
    #[serde(rename = "parentDir")]
    pub parent_dir: String,
}

#[derive(Deserialize)]
pub struct ReadQuery {
    pub path: Option<String>,
}

/// Minimal ComicInfo.xml representation.
#[derive(Deserialize, Default)]
#[serde(rename = "ComicInfo", default)]
struct ComicInfo {
    #[serde(rename = "Title")]
    title: String,
    #[serde(rename = "Series")]
    series: String,
}

pub async fn read_handler(
    query: web::Query<ReadQuery>,
    req: HttpRequest,
    config: web::Data<Config>,
) -> impl Responder {
    let client_path = match &query.path {
        Some(p) => p.clone(),
        None => return HttpResponse::BadRequest().body("Missing 'path' query parameter"),
    };

    let real_path = match get_real_path(&client_path, &config) {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    let mut builder = HttpResponse::Ok();
    if check_file_cache(&real_path, &req, &mut builder) {
        return builder.finish();
    }

    let (_, parent_dir) = get_parent_dir(&real_path, &config);

    let extension = get_extension(real_path.to_str().unwrap_or(""));

    match extension.as_str() {
        "cbz" => {
            let abs_path = real_path.to_string_lossy().to_string();
            let result =
                web::block(move || read_cbz(&abs_path, &client_path, parent_dir))
                    .await;

            match result {
                Ok(Ok(read_info)) => {
                    apply_cache_headers(&real_path, &mut builder);
                    builder.content_type("application/json").json(read_info)
                }
                Ok(Err(e)) => {
                    log::error!("read_cbz error: {}", e);
                    HttpResponse::InternalServerError().body(e)
                }
                Err(e) => {
                    log::error!("blocking error: {}", e);
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        _ => HttpResponse::BadRequest().body("Unsupported file type"),
    }
}

fn read_cbz(
    abs_path: &str,
    client_path: &str,
    parent_dir: String,
) -> Result<ReadInfo, String> {
    let file = File::open(abs_path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read CBZ archive: {}", e))?;

    let mut comic_title = String::new();

    // Try to read ComicInfo.xml without decompressing other entries.
    if let Ok(mut comic_info_entry) = archive.by_name("ComicInfo.xml") {
        let mut xml_data = Vec::new();
        comic_info_entry
            .read_to_end(&mut xml_data)
            .map_err(|e| format!("Failed to read ComicInfo.xml: {}", e))?;

        if let Ok(info) = quick_xml::de::from_reader::<_, ComicInfo>(xml_data.as_slice()) {
            if !info.title.is_empty() {
                comic_title = info.title.clone();
                if !info.series.is_empty() {
                    comic_title.push_str(" - ");
                    comic_title.push_str(&info.series);
                }
            }
        }
    }

    // Collect image file names from the archive (only reads central directory).
    let pages = get_page_list_from_archive(&mut archive)?;

    let page_cnt = pages.len();

    Ok(ReadInfo {
        comic_title,
        pages,
        path: client_path.to_string(),
        page_cnt,
        parent_dir,
    })
}

/// Returns a naturally-sorted list of image pages found in the archive.
/// This only reads the central directory – no file content is decompressed.
pub fn get_page_list_from_archive(
    archive: &mut zip::ZipArchive<File>,
) -> Result<Vec<PageInfo>, String> {
    let mut names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index_raw(i).ok().map(|f| f.name().to_string()))
        .filter(|name| is_supported_image(&get_extension(name)))
        .collect();

    names.sort_by(|a, b| natord::compare(a, b));

    let pages = names
        .into_iter()
        .enumerate()
        .map(|(i, name)| PageInfo {
            page_no: i + 1,
            image_file: name,
        })
        .collect();

    Ok(pages)
}
