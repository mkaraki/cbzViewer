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

/// Handles a read request for a comic file and returns metadata and a page list.
///
/// Extracts the `path` query parameter, resolves it to a real filesystem path, checks HTTP cache headers, and for `.cbz` archives returns a JSON `ReadInfo` describing the comic (title, ordered pages, path, page count, and parent directory). If the `path` parameter is missing or the file extension is unsupported, returns `400 Bad Request`. If processing the archive fails, returns `500 Internal Server Error`.
///
/// # Returns
///
/// An HTTP response: `200 OK` with JSON `ReadInfo` on success; `400 Bad Request` when `path` is missing or the file type is unsupported; `500 Internal Server Error` on internal failures.
///
/// # Examples
///
/// ```
/// # use actix_web::http::StatusCode;
/// # async fn example() {
/// // Construct a query with a client path and call the handler in an async context.
/// // Note: types like `Config` and application setup are omitted for brevity.
/// // let query = web::Query(ReadQuery { path: Some("/comics/example.cbz".into()) });
/// // let resp = read_handler(query, HttpRequest::default(), web::Data::new(Config::default())).await;
/// // assert_eq!(resp.respond_to(&HttpRequest::default()).status(), StatusCode::OK);
/// # }
/// ```
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
                    sentry::capture_message(&e, sentry::Level::Error);
                    HttpResponse::InternalServerError().body(e)
                }
                Err(e) => {
                    sentry::capture_error(&e);
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        _ => HttpResponse::BadRequest().body("Unsupported file type"),
    }
}

/// Reads a CBZ archive and returns metadata and a naturally-sorted list of image pages.
///
/// Attempts to open the archive at `abs_path`, extract an optional `ComicInfo.xml` title/series,
/// and build a list of supported image file entries (using the archive central directory only).
/// The returned `ReadInfo` includes the resolved comic title (if any), page list, original
/// `client_path`, page count, and `parent_dir` metadata.
///
/// # Parameters
///
/// - `abs_path`: Absolute filesystem path to the `.cbz` file to read.
/// - `client_path`: The original path provided by the client; copied into the returned `ReadInfo`.
/// - `parent_dir`: Parent-directory metadata to include in the returned `ReadInfo`.
///
/// # Returns
///
/// `Ok(ReadInfo)` with parsed comic metadata and pages on success, or `Err(String)` with a
/// human-readable error message on failure.
///
/// # Examples
///
/// ```no_run
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let info = read_cbz("/data/comics/example.cbz", "/comics/example.cbz", "comics".to_string())?;
/// println!("Title: {}", info.comic_title);
/// assert!(info.page_cnt == info.pages.len());
/// # Ok(())
/// # }
/// ```
#[tracing::instrument]
fn read_cbz(
    abs_path: &str,
    client_path: &str,
    parent_dir: String,
) -> Result<ReadInfo, String> {
    tracing::trace!(
        "CALL read::read_cbz({}, {}, {})",
        abs_path.to_owned(), client_path.to_owned(), parent_dir
    );

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

/// Build a naturally sorted list of image pages from a ZIP archive's central directory.
///
/// This inspects the archive entries without decompressing file contents and returns
/// a Vec<PageInfo> with 1-based page numbers for entries whose extensions indicate supported images.
///
/// # Examples
///
/// ```
/// use std::fs::File;
/// use std::io::Write;
/// use std::path::PathBuf;
///
/// // Create a temporary zip file with a few entries.
/// let mut path = std::env::temp_dir();
/// path.push("get_page_list_test.zip");
/// let file = File::create(&path).expect("create temp zip");
///
/// let mut zipw = zip::ZipWriter::new(file);
/// let options = zip::write::FileOptions::default();
/// zipw.start_file("b.png", options).unwrap();
/// zipw.write_all(b"pngdata").unwrap();
/// zipw.start_file("a.jpg", options).unwrap();
/// zipw.write_all(b"jpgdata").unwrap();
/// zipw.start_file("ignore.txt", options).unwrap();
/// zipw.write_all(b"text").unwrap();
/// let file = zipw.finish().unwrap();
///
/// // Reopen for reading and build the page list.
/// let mut reader = File::open(&path).expect("open temp zip");
/// let mut archive = zip::ZipArchive::new(reader).expect("read zip archive");
/// let pages = crate::read::get_page_list_from_archive(&mut archive).expect("list pages");
///
/// assert_eq!(pages.len(), 2);
/// assert_eq!(pages[0].page_no, 1);
/// assert_eq!(pages[0].image_file, "a.jpg");
/// assert_eq!(pages[1].page_no, 2);
/// assert_eq!(pages[1].image_file, "b.png");
/// ```
#[tracing::instrument]
pub fn get_page_list_from_archive(
    archive: &mut zip::ZipArchive<File>,
) -> Result<Vec<PageInfo>, String> {
    tracing::trace!("CALL read::get_page_list_from_archive(archive)");

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
