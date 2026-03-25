use std::fs::File;
use std::io::Read;

use actix_web::{web, HttpRequest, HttpResponse, Responder};
use crate::config::Config;
use crate::pathutils::{
    apply_cache_headers, check_file_cache, get_content_type, get_extension, get_real_path,
    is_supported_image,
};

#[derive(serde::Deserialize)]
pub struct ImgQuery {
    pub path: Option<String>,
    pub f: Option<String>,
    pub thumb: Option<String>,
    pub size: Option<String>,
}

pub async fn img_handler(
    query: web::Query<ImgQuery>,
    req: HttpRequest,
    config: web::Data<Config>,
) -> impl Responder {
    let client_path = match &query.path {
        Some(p) => p.clone(),
        None => return HttpResponse::BadRequest().body("Missing 'path' query parameter"),
    };
    let query_file = match &query.f {
        Some(f) => f.clone(),
        None => return HttpResponse::BadRequest().body("Missing 'f' query parameter"),
    };

    // Determine requested size: thumbnail = 100px, explicit size, or -1 (original).
    let size: i32 = if query.thumb.is_some() {
        100
    } else if let Some(s) = &query.size {
        s.parse::<i32>().unwrap_or(-1)
    } else {
        -1
    };

    let real_path = match get_real_path(&client_path, &config) {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    let mut builder = HttpResponse::Ok();
    if check_file_cache(&real_path, &req, &mut builder) {
        return builder.finish();
    }

    let extension = get_extension(real_path.to_str().unwrap_or(""));

    match extension.as_str() {
        "cbz" => {
            let request_ext = get_extension(&query_file);

            if !is_supported_image(&request_ext) {
                return HttpResponse::BadRequest().body("Not a supported image format");
            }

            // Validate the zip-internal path to prevent any zip-slip style issue.
            if !is_safe_zip_path(&query_file) {
                return HttpResponse::BadRequest().body("Invalid image path");
            }

            let abs_path = real_path.to_string_lossy().to_string();
            let query_file_clone = query_file.clone();

            let result = web::block(move || {
                serve_cbz_image(&abs_path, &query_file_clone, size)
            })
            .await;

            match result {
                Ok(Ok((data, content_type))) => {
                    apply_cache_headers(&real_path, &mut builder);
                    builder.content_type(content_type).body(data)
                }
                Ok(Err(e)) => {
                    sentry::capture_message(&e, sentry::Level::Error);
                    HttpResponse::InternalServerError().into()
                }
                Err(e) => {
                    sentry::capture_error(&e);
                    HttpResponse::InternalServerError().into()
                }
            }
        }
        _ => HttpResponse::BadRequest().body("Unsupported file type"),
    }
}

/// Ensures a zip-internal path does not attempt to escape the archive.
/// Rejects absolute paths, paths containing `..` components, and any
/// platform-specific absolute prefixes (e.g. Windows drive letters).
fn is_safe_zip_path(path: &str) -> bool {
    tracing::trace!("CALL is_safe_zip_path: {}", path);

    // Reject clearly unsafe patterns early.
    if path.starts_with('/') || path.starts_with('\\') {
        return false;
    }
    // Reject Windows-style absolute paths like "C:\..." or "C:/...".
    if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        return false;
    }
    // Normalise to a PathBuf and verify no component is `..`.
    let p = std::path::PathBuf::from(path);
    for component in p.components() {
        match component {
            std::path::Component::ParentDir => return false,
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return false,
            _ => {}
        }
    }
    true
}

/// Reads an image entry from a CBZ archive and optionally resizes it.
/// Returns `(image_bytes, content_type)`.
#[tracing::instrument]
fn serve_cbz_image(
    cbz_path: &str,
    image_name: &str,
    size: i32,
) -> Result<(Vec<u8>, &'static str), String> {
    tracing::trace!("CALL img::serve_cbz_image({}, {}, {})", cbz_path, image_name, size);

    let file = File::open(cbz_path).map_err(|e| format!("Failed to open CBZ: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read CBZ: {}", e))?;

    let mut entry = archive
        .by_name(image_name)
        .map_err(|e| format!("Image not found in CBZ: {}", e))?;

    // Stream only this one entry into memory.
    let mut raw = Vec::with_capacity(entry.size() as usize);
    entry
        .read_to_end(&mut raw)
        .map_err(|e| format!("Failed to read image data: {}", e))?;

    if size == -1 {
        // Serve the original bytes without decoding.
        let ext = get_extension(image_name);
        let content_type = get_content_type(&ext);
        return Ok((raw, content_type));
    }

    // Decode → resize → re-encode as WebP.
    let img = image::load_from_memory(&raw)
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    let resized = resize_image(img, size as u32);

    let quality: u8 = if size < 320 { 40 } else { 85 };
    let jpeg_bytes = encode_jpeg(&resized, quality)
        .map_err(|e| format!("Failed to encode JPEG: {}", e))?;

    Ok((jpeg_bytes, "image/jpeg"))
}

/// Resizes `img` to `target_width` pixels wide, preserving aspect ratio,
/// using the Lanczos3 filter from `fast_image_resize`.
#[tracing::instrument]
fn resize_image(img: image::DynamicImage, target_width: u32) -> image::DynamicImage {
    tracing::trace!("CALL img::resize_image(img, {})", target_width);

    use fast_image_resize::{
        images::{Image, ImageRef},
        FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer,
    };

    let src_width = img.width();
    let src_height = img.height();

    if src_width == 0 || target_width == 0 {
        return img;
    }

    // Skip resize when the image is already smaller than the target.
    if src_width <= target_width {
        return img;
    }

    let target_height =
        ((src_height as f64 * target_width as f64) / src_width as f64).round() as u32;
    let target_height = target_height.max(1);

    let rgba = img.to_rgba8();
    let raw = rgba.as_raw();

    let src = match ImageRef::new(src_width, src_height, raw, PixelType::U8x4) {
        Ok(s) => s,
        Err(e) => {
            sentry::capture_error(&e);
            tracing::error!("fast_image_resize ImageRef error");
            return img;
        }
    };

    let mut dst = Image::new(target_width, target_height, PixelType::U8x4);

    let mut resizer = Resizer::new();
    let options = ResizeOptions::new()
        .resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));

    if let Err(e) = resizer.resize(&src, &mut dst, &options) {
        sentry::capture_error(&e);
        tracing::error!("fast_image_resize resize error");
        return img;
    }

    let dst_rgba = image::RgbaImage::from_raw(target_width, target_height, dst.into_vec())
        .expect("resize produced invalid buffer size");

    image::DynamicImage::ImageRgba8(dst_rgba)
}

/// Encodes a `DynamicImage` as JPEG with the given quality (0–100).
#[tracing::instrument]
fn encode_jpeg(img: &image::DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    tracing::trace!("CALL img::encode_jpeg(img, {})", quality);

    let mut output = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, quality);
    encoder
        .encode_image(img)
        .map_err(|e| format!("JPEG encode error: {}", e))?;
    Ok(output)
}
