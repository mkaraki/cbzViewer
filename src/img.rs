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

/// Serves an image from a filesystem path or a CBZ archive, optionally resizing it.
///
/// Validates that `path` and `f` query parameters are present, rejects unsafe archive entry paths,
/// checks and applies HTTP cache headers, and supports `thumb` (forces 100px width) or `size` to
/// request a resized JPEG; when no size is requested the original entry bytes and content type are returned.
///
/// # Returns
/// An HTTP response containing the image bytes and appropriate `Content-Type` on success; `400 Bad Request` for invalid input (missing parameters, unsafe archive paths, or unsupported formats); or `500 Internal Server Error` for processing failures.
///
/// # Examples
///
/// ```
/// // Construct a query and call the handler (example sketch; actual invocation requires Actix runtime)
/// // let query = web::Query(ImgQuery { path: Some("comics.cbz".into()), f: Some("page1.jpg".into()), thumb: None, size: Some("200".into()) });
/// // let resp = img_handler(query, req, config).await;
/// ```
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

/// Validate a zip-internal path to prevent archive traversal.
///
/// This function checks that the provided `path` is safe to pass to
/// `zip::ZipArchive::by_name` by rejecting:
/// - paths that start with `/` or `\` (absolute POSIX/UNC paths),
/// - Windows-style absolute prefixes like `C:\...` or `C:/...`,
/// - any path component equal to `..`, and
/// - any platform root or prefix components.
///
/// # Returns
///
/// `true` if the path does not contain absolute roots, drive-letter prefixes,
/// or parent-directory (`..`) components; `false` otherwise.
///
/// # Examples
///
/// ```
/// assert!(is_safe_zip_path("images/page1.jpg"));
/// assert!(is_safe_zip_path("nested/dir/image.png"));
/// assert!(!is_safe_zip_path("/etc/passwd"));
/// assert!(!is_safe_zip_path("C:\\Windows\\system32\\cmd.exe"));
/// assert!(!is_safe_zip_path("foo/../bar.jpg"));
/// ```
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

/// Read an image entry from a CBZ (zip) archive and return its bytes along with the MIME content type.
///
/// If `size == -1`, this function returns the original entry bytes and a content type derived from the
/// entry's extension. If `size != -1`, the entry is decoded, resized to the requested width while
/// preserving aspect ratio, re-encoded as JPEG, and returned with content type `"image/jpeg"`.
///
/// # Parameters
///
/// - `cbz_path`: filesystem path to the CBZ file.
/// - `image_name`: path/name of the entry inside the archive.
/// - `size`: target width in pixels; `-1` requests the original image (no decoding/resizing).
///
/// # Returns
///
/// On success returns `Ok((bytes, content_type))` where `bytes` are the image data to send and
/// `content_type` is the MIME type (e.g., `"image/jpeg"`). On failure returns `Err` with an error message.
///
/// # Examples
///
/// ```
/// use std::io::Write;
/// use zip::write::FileOptions;
///
/// // Create a temporary CBZ (zip) with a single entry "img.jpg".
/// let dir = tempfile::tempdir().unwrap();
/// let cbz_path = dir.path().join("test.cbz");
/// let f = std::fs::File::create(&cbz_path).unwrap();
/// let mut zip = zip::ZipWriter::new(f);
/// zip.start_file("img.jpg", FileOptions::default()).unwrap();
/// zip.write_all(b"fakejpegdata").unwrap();
/// zip.finish().unwrap();
///
/// // Serve the original bytes from the archive.
/// let (bytes, content_type) = crate::img::serve_cbz_image(cbz_path.to_str().unwrap(), "img.jpg", -1).unwrap();
/// assert_eq!(content_type, "image/jpeg");
/// assert!(!bytes.is_empty());
/// ```
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

    // Decode → resize → re-encode as JPEG.
    let img = image::load_from_memory(&raw)
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    let resized = resize_image(img, size as u32);

    let quality: u8 = if size < 320 { 40 } else { 85 };
    let jpeg_bytes = encode_jpeg(&resized, quality)
        .map_err(|e| format!("Failed to encode JPEG: {}", e))?;

    Ok((jpeg_bytes, "image/jpeg"))
}

/// Resize an image to the specified target width while preserving aspect ratio.
///
/// Returns the original image unmodified if the source width is zero, the target width is zero,
/// or the source width is less than or equal to the target width (no upscaling).
///
/// Uses the Lanczos3 filter from `fast_image_resize` for high-quality downscaling. If image
/// conversion or the resizing operation fails, the error is captured to Sentry and the original
/// image is returned.
///
/// # Examples
///
/// ```
/// use image::{DynamicImage, RgbaImage};
///
/// // 100x50 source image
/// let src = DynamicImage::ImageRgba8(RgbaImage::new(100, 50));
/// let out = resize_image(src, 50);
/// assert_eq!(out.width(), 50);
/// ```
#[tracing::instrument]
fn resize_image(img: image::DynamicImage, target_width: u32) -> image::DynamicImage {
    tracing::trace!("CALL img::resize_image(img, {})", target_width);

    use fast_image_resize::{
        images::{Image},
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
    drop(img);

    let mut dst = Image::new(target_width, target_height, PixelType::U8x4);

    let mut resizer = Resizer::new();
    let options = ResizeOptions::new()
        .resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));

    let res = resizer.resize(&rgba, &mut dst, &options);
    if res.is_err() {
        sentry::capture_error(&res.err().unwrap());
        tracing::error!("fast_image_resize resize error");
        return rgba.into();
    }

    let dst_rgba = image::RgbaImage::from_raw(target_width, target_height, dst.into_vec());
    if dst_rgba.is_none() {
        sentry::capture_message("Failed to create RgbaImage from resized bytes", sentry::Level::Error);
        tracing::error!("Failed to create Image Object from resized RgbaImage bytes");
        return rgba.into();
    }
    let dst_rgba = dst_rgba.unwrap();
    drop(rgba);

    image::DynamicImage::ImageRgba8(dst_rgba)
}

/// Encode a `DynamicImage` as a JPEG at the specified quality.
///
/// Returns a `Vec<u8>` with the JPEG-encoded bytes on success, or an error `String` if encoding fails.
///
/// # Examples
///
/// ```
/// use image::{DynamicImage, RgbImage, Rgb};
///
/// let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(1, 1, Rgb([255, 0, 0])));
/// let jpg = crate::img::encode_jpeg(&img, 80).unwrap();
/// assert!(jpg.len() > 0);
/// ```
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbaImage, RgbImage, Rgb, Rgba};
    use std::io::Write;

    // ── is_safe_zip_path ────────────────────────────────────────────────────────

    #[test]
    fn test_is_safe_zip_path_simple_file() {
        assert!(is_safe_zip_path("page1.jpg"));
    }

    #[test]
    fn test_is_safe_zip_path_nested() {
        assert!(is_safe_zip_path("images/page1.jpg"));
    }

    #[test]
    fn test_is_safe_zip_path_deep_nested() {
        assert!(is_safe_zip_path("a/b/c/image.png"));
    }

    #[test]
    fn test_is_safe_zip_path_rejects_absolute_posix() {
        assert!(!is_safe_zip_path("/etc/passwd"));
    }

    #[test]
    fn test_is_safe_zip_path_rejects_absolute_backslash() {
        assert!(!is_safe_zip_path("\\Windows\\system32"));
    }

    #[test]
    fn test_is_safe_zip_path_rejects_parent_dir() {
        assert!(!is_safe_zip_path("../secret.txt"));
    }

    #[test]
    fn test_is_safe_zip_path_rejects_parent_dir_nested() {
        assert!(!is_safe_zip_path("foo/../bar.jpg"));
    }

    #[test]
    fn test_is_safe_zip_path_rejects_windows_drive_letter_backslash() {
        assert!(!is_safe_zip_path("C:\\Windows\\system32\\cmd.exe"));
    }

    #[test]
    fn test_is_safe_zip_path_rejects_windows_drive_letter_slash() {
        assert!(!is_safe_zip_path("C:/Windows/System32"));
    }

    #[test]
    fn test_is_safe_zip_path_empty_string() {
        // An empty path has no dangerous components; it is safe.
        assert!(is_safe_zip_path(""));
    }

    #[test]
    fn test_is_safe_zip_path_single_dot() {
        // A CurDir component ("." itself) is not ParentDir and not a root, so safe.
        assert!(is_safe_zip_path("./image.jpg"));
    }

    // ── resize_image ────────────────────────────────────────────────────────────

    #[test]
    fn test_resize_image_downscales_width() {
        let src = DynamicImage::ImageRgba8(RgbaImage::new(100, 50));
        let out = resize_image(src, 50);
        assert_eq!(out.width(), 50);
    }

    #[test]
    fn test_resize_image_preserves_aspect_ratio() {
        let src = DynamicImage::ImageRgba8(RgbaImage::new(200, 100));
        let out = resize_image(src, 100);
        // 200x100 scaled to width 100 → height should be 50
        assert_eq!(out.width(), 100);
        assert_eq!(out.height(), 50);
    }

    #[test]
    fn test_resize_image_no_upscale() {
        // Source width (50) <= target width (200): return original unchanged.
        let src = DynamicImage::ImageRgba8(RgbaImage::new(50, 30));
        let out = resize_image(src, 200);
        assert_eq!(out.width(), 50);
        assert_eq!(out.height(), 30);
    }

    #[test]
    fn test_resize_image_equal_width_returns_original() {
        let src = DynamicImage::ImageRgba8(RgbaImage::new(100, 60));
        let out = resize_image(src, 100);
        assert_eq!(out.width(), 100);
        assert_eq!(out.height(), 60);
    }

    #[test]
    fn test_resize_image_zero_target_returns_original() {
        let src = DynamicImage::ImageRgba8(RgbaImage::new(100, 50));
        let out = resize_image(src, 0);
        assert_eq!(out.width(), 100);
        assert_eq!(out.height(), 50);
    }

    #[test]
    fn test_resize_image_zero_src_width_returns_original() {
        // An image with 0-width is degenerate; the function should return it unmodified.
        let src = DynamicImage::ImageRgba8(RgbaImage::new(0, 0));
        let out = resize_image(src, 50);
        assert_eq!(out.width(), 0);
    }

    // ── encode_jpeg ─────────────────────────────────────────────────────────────

    #[test]
    fn test_encode_jpeg_produces_nonempty_bytes() {
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(4, 4, Rgb([128, 64, 32])));
        let jpg = encode_jpeg(&img, 80).unwrap();
        assert!(!jpg.is_empty());
    }

    #[test]
    fn test_encode_jpeg_starts_with_jpeg_magic_bytes() {
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(4, 4, Rgb([0, 0, 0])));
        let jpg = encode_jpeg(&img, 75).unwrap();
        // JPEG files start with the SOI marker: 0xFF 0xD8
        assert!(jpg.len() >= 2);
        assert_eq!(jpg[0], 0xFF);
        assert_eq!(jpg[1], 0xD8);
    }

    #[test]
    fn test_encode_jpeg_quality_1_produces_smaller_output_than_quality_100() {
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(64, 64, Rgb([200, 150, 100])));
        let low_q = encode_jpeg(&img, 1).unwrap();
        let high_q = encode_jpeg(&img, 100).unwrap();
        assert!(
            low_q.len() < high_q.len(),
            "quality 1 output ({}) should be smaller than quality 100 ({})",
            low_q.len(),
            high_q.len()
        );
    }

    #[test]
    fn test_encode_jpeg_rgba_image() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 255])));
        let jpg = encode_jpeg(&img, 80).unwrap();
        assert!(!jpg.is_empty());
        assert_eq!(jpg[0], 0xFF);
        assert_eq!(jpg[1], 0xD8);
    }

    // ── serve_cbz_image ─────────────────────────────────────────────────────────

    fn make_cbz_with_jpeg(dir: &std::path::Path, entry_name: &str, image_data: Vec<u8>) -> std::path::PathBuf {
        use zip::write::SimpleFileOptions;

        let cbz_path = dir.join("test.cbz");
        let f = std::fs::File::create(&cbz_path).unwrap();
        let mut zip = zip::ZipWriter::new(f);
        zip.start_file(entry_name, SimpleFileOptions::default()).unwrap();
        zip.write_all(&image_data).unwrap();
        zip.finish().unwrap();
        cbz_path
    }

    fn make_minimal_jpeg() -> Vec<u8> {
        // Create a tiny 4×4 RGB image and encode it as JPEG.
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(4, 4, Rgb([200, 100, 50])));
        encode_jpeg(&img, 80).unwrap()
    }

    #[test]
    fn test_serve_cbz_image_original_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let jpeg_data = make_minimal_jpeg();
        let cbz_path = make_cbz_with_jpeg(dir.path(), "page.jpg", jpeg_data.clone());

        let (data, ct) = serve_cbz_image(cbz_path.to_str().unwrap(), "page.jpg", -1).unwrap();
        assert_eq!(ct, "image/jpeg");
        assert_eq!(data, jpeg_data);
    }

    #[test]
    fn test_serve_cbz_image_resized_returns_jpeg_mime() {
        let dir = tempfile::tempdir().unwrap();
        let jpeg_data = make_minimal_jpeg();
        let cbz_path = make_cbz_with_jpeg(dir.path(), "page.jpg", jpeg_data);

        let (data, ct) = serve_cbz_image(cbz_path.to_str().unwrap(), "page.jpg", 2).unwrap();
        assert_eq!(ct, "image/jpeg");
        assert!(!data.is_empty());
        // Verify it's still a valid JPEG
        assert_eq!(data[0], 0xFF);
        assert_eq!(data[1], 0xD8);
    }

    #[test]
    fn test_serve_cbz_image_missing_entry_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let cbz_path = make_cbz_with_jpeg(dir.path(), "page.jpg", make_minimal_jpeg());

        let result = serve_cbz_image(cbz_path.to_str().unwrap(), "nonexistent.jpg", -1);
        assert!(result.is_err(), "Expected Err for missing archive entry");
    }

    #[test]
    fn test_serve_cbz_image_nonexistent_file_returns_err() {
        let result = serve_cbz_image("/nonexistent/path/file.cbz", "page.jpg", -1);
        assert!(result.is_err(), "Expected Err for nonexistent CBZ file");
    }

    #[test]
    fn test_serve_cbz_image_png_original_bytes_content_type() {
        use zip::write::SimpleFileOptions;
        // Create a small PNG to verify content-type mapping.
        let mut png_data = Vec::new();
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(4, 4, Rgb([0, 255, 0])));
        img.write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let cbz_path = dir.path().join("test.cbz");
        let f = std::fs::File::create(&cbz_path).unwrap();
        let mut zip = zip::ZipWriter::new(f);
        zip.start_file("page.png", SimpleFileOptions::default()).unwrap();
        zip.write_all(&png_data).unwrap();
        zip.finish().unwrap();

        let (_, ct) = serve_cbz_image(cbz_path.to_str().unwrap(), "page.png", -1).unwrap();
        assert_eq!(ct, "image/png");
    }

    #[test]
    fn test_serve_cbz_image_small_thumb_uses_low_quality() {
        // size < 320 uses quality=40; size >= 320 uses quality=85.
        // Both should produce valid JPEG output; we only verify the call succeeds.
        let dir = tempfile::tempdir().unwrap();
        let jpeg_data = make_minimal_jpeg();
        let cbz_path = make_cbz_with_jpeg(dir.path(), "page.jpg", jpeg_data);

        // size=100 (thumb) → quality 40
        let (data, ct) = serve_cbz_image(cbz_path.to_str().unwrap(), "page.jpg", 100).unwrap();
        assert_eq!(ct, "image/jpeg");
        assert!(!data.is_empty());
    }

    #[test]
    fn test_serve_cbz_image_invalid_cbz_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let bad_cbz = dir.path().join("bad.cbz");
        std::fs::write(&bad_cbz, b"this is not a zip file").unwrap();

        let result = serve_cbz_image(bad_cbz.to_str().unwrap(), "page.jpg", -1);
        assert!(result.is_err(), "Expected Err for corrupt CBZ");
    }
}