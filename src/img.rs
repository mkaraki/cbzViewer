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

    // ───────────────────────── is_safe_zip_path ─────────────────────────

    #[test]
    fn safe_path_simple_filename() {
        assert!(is_safe_zip_path("page1.jpg"));
    }

    #[test]
    fn safe_path_nested() {
        assert!(is_safe_zip_path("subdir/page1.jpg"));
    }

    #[test]
    fn safe_path_deeply_nested() {
        assert!(is_safe_zip_path("a/b/c/d/image.png"));
    }

    #[test]
    fn safe_path_with_dot_filename() {
        // A leading dot in the *name* is fine (e.g. hidden files)
        assert!(is_safe_zip_path("subdir/.hidden.jpg"));
    }

    #[test]
    fn unsafe_path_absolute_posix() {
        assert!(!is_safe_zip_path("/etc/passwd"));
    }

    #[test]
    fn unsafe_path_absolute_backslash() {
        assert!(!is_safe_zip_path("\\windows\\system32"));
    }

    #[test]
    fn unsafe_path_windows_drive_letter_backslash() {
        assert!(!is_safe_zip_path("C:\\Windows\\system32\\cmd.exe"));
    }

    #[test]
    fn unsafe_path_windows_drive_letter_slash() {
        assert!(!is_safe_zip_path("C:/Windows/system32/cmd.exe"));
    }

    #[test]
    fn unsafe_path_parent_dir_component() {
        assert!(!is_safe_zip_path("foo/../bar.jpg"));
    }

    #[test]
    fn unsafe_path_starts_with_parent_dir() {
        assert!(!is_safe_zip_path("../outside.jpg"));
    }

    #[test]
    fn unsafe_path_only_parent_dir() {
        assert!(!is_safe_zip_path(".."));
    }

    #[test]
    fn safe_path_empty_string() {
        // An empty path has no unsafe components; behaviour is consistent.
        assert!(is_safe_zip_path(""));
    }

    #[test]
    fn safe_path_current_dir_dot() {
        // A single "." (current dir) is treated as a normal component.
        assert!(is_safe_zip_path("."));
    }

    #[test]
    fn unsafe_path_middle_parent_dir() {
        assert!(!is_safe_zip_path("a/b/../../etc/passwd"));
    }

    // ───────────────────────── resize_image ─────────────────────────────

    fn make_rgba_image(w: u32, h: u32) -> DynamicImage {
        let img = RgbaImage::from_pixel(w, h, Rgba([128u8, 64, 32, 255]));
        DynamicImage::ImageRgba8(img)
    }

    #[test]
    fn resize_image_downscales_width() {
        let src = make_rgba_image(200, 100);
        let out = resize_image(src, 50);
        assert_eq!(out.width(), 50);
        // Height should be proportional: 100 * 50/200 = 25
        assert_eq!(out.height(), 25);
    }

    #[test]
    fn resize_image_no_upscale_when_target_larger() {
        let src = make_rgba_image(50, 50);
        let out = resize_image(src, 200);
        // Source is smaller than target; original is returned unchanged.
        assert_eq!(out.width(), 50);
        assert_eq!(out.height(), 50);
    }

    #[test]
    fn resize_image_no_change_when_equal_width() {
        let src = make_rgba_image(100, 100);
        let out = resize_image(src, 100);
        assert_eq!(out.width(), 100);
        assert_eq!(out.height(), 100);
    }

    #[test]
    fn resize_image_zero_target_width_returns_original() {
        let src = make_rgba_image(100, 50);
        let out = resize_image(src, 0);
        assert_eq!(out.width(), 100);
        assert_eq!(out.height(), 50);
    }

    #[test]
    fn resize_image_large_aspect_ratio_preserved() {
        // Wide image: 400x100, downscale to width 100 → expected height 25
        let src = make_rgba_image(400, 100);
        let out = resize_image(src, 100);
        assert_eq!(out.width(), 100);
        assert_eq!(out.height(), 25);
    }

    #[test]
    fn resize_image_tall_aspect_ratio_preserved() {
        // Tall image: 100x400, downscale to width 50 → expected height 200
        let src = make_rgba_image(100, 400);
        let out = resize_image(src, 50);
        assert_eq!(out.width(), 50);
        assert_eq!(out.height(), 200);
    }

    #[test]
    fn resize_image_minimum_height_is_one() {
        // Extremely wide image where computed height would be < 1
        // 1000x1, target width 1 → height = round(1 * 1 / 1000) = 0, clamped to 1
        let src = make_rgba_image(1000, 1);
        let out = resize_image(src, 1);
        assert_eq!(out.width(), 1);
        assert!(out.height() >= 1);
    }

    // ───────────────────────── encode_jpeg ──────────────────────────────

    fn make_rgb_image(w: u32, h: u32) -> DynamicImage {
        let img = RgbImage::from_pixel(w, h, Rgb([200u8, 100, 50]));
        DynamicImage::ImageRgb8(img)
    }

    #[test]
    fn encode_jpeg_produces_valid_jpeg_magic_bytes() {
        let img = make_rgb_image(10, 10);
        let bytes = encode_jpeg(&img, 80).expect("encoding should succeed");
        // JPEG files start with 0xFF 0xD8
        assert!(bytes.len() >= 2, "JPEG output must have at least 2 bytes");
        assert_eq!(bytes[0], 0xFF, "first byte should be 0xFF");
        assert_eq!(bytes[1], 0xD8, "second byte should be 0xD8 (JPEG SOI)");
    }

    #[test]
    fn encode_jpeg_quality_100_nonempty() {
        let img = make_rgb_image(4, 4);
        let bytes = encode_jpeg(&img, 100).expect("quality 100 should succeed");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn encode_jpeg_quality_1_nonempty() {
        let img = make_rgb_image(4, 4);
        let bytes = encode_jpeg(&img, 1).expect("quality 1 should succeed");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn encode_jpeg_high_quality_larger_than_low_quality() {
        // Build a 64×64 image with highly varied pixel values (checkerboard-style)
        // so that the JPEG codec has enough entropy to show a meaningful quality
        // difference in output size.
        let mut pixels = Vec::with_capacity(64 * 64 * 3);
        for y in 0u8..64 {
            for x in 0u8..64 {
                let v = x.wrapping_mul(4).wrapping_add(y.wrapping_mul(7));
                pixels.push(v);
                pixels.push(v.wrapping_add(80));
                pixels.push(255u8.wrapping_sub(v));
            }
        }
        let img = DynamicImage::ImageRgb8(
            RgbImage::from_raw(64, 64, pixels).expect("valid pixel buffer"),
        );
        let high = encode_jpeg(&img, 95).unwrap();
        let low = encode_jpeg(&img, 5).unwrap();
        // Higher quality generally means larger output for non-trivial images.
        assert!(
            high.len() > low.len(),
            "high quality ({} bytes) should be larger than low quality ({} bytes)",
            high.len(),
            low.len()
        );
    }

    #[test]
    fn encode_jpeg_from_rgba_image() {
        let img = make_rgba_image(8, 8);
        let bytes = encode_jpeg(&img, 75).expect("encoding RGBA should succeed");
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xD8);
    }

    // ───────────────────────── serve_cbz_image ──────────────────────────

    fn create_test_cbz_with_png(png_bytes: &[u8]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let cbz_path = dir.path().join("test.cbz");
        let file = std::fs::File::create(&cbz_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("image.png", options).unwrap();
        zip.write_all(png_bytes).unwrap();
        zip.finish().unwrap();
        dir
    }

    fn create_small_png() -> Vec<u8> {
        // Encode a 2x2 RGBA image as PNG into a buffer.
        let img = RgbaImage::from_pixel(2, 2, Rgba([255u8, 0, 0, 255]));
        let dyn_img = DynamicImage::ImageRgba8(img);
        let mut buf = Vec::new();
        dyn_img
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    #[test]
    fn serve_cbz_image_original_returns_correct_content_type() {
        let png_bytes = create_small_png();
        let dir = create_test_cbz_with_png(&png_bytes);
        let cbz_path = dir.path().join("test.cbz");

        let (bytes, content_type) =
            serve_cbz_image(cbz_path.to_str().unwrap(), "image.png", -1)
                .expect("serve should succeed");

        assert_eq!(content_type, "image/png");
        assert!(!bytes.is_empty());
        // The original PNG bytes should round-trip unchanged.
        assert_eq!(bytes, png_bytes);
    }

    #[test]
    fn serve_cbz_image_with_resize_returns_jpeg() {
        // Use the real CBZ fixture so we have a proper image to decode/resize.
        let cbz_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/books/tests/Testing Introduction 01.cbz"
        );
        let (bytes, content_type) =
            serve_cbz_image(cbz_path, "subdir_1/0.png", 50)
                .expect("resize should succeed");

        assert_eq!(content_type, "image/jpeg");
        assert!(!bytes.is_empty());
        // Verify JPEG magic bytes.
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xD8);
    }

    #[test]
    fn serve_cbz_image_original_jpeg_returns_correct_content_type() {
        let cbz_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/books/tests/Testing Introduction 01.cbz"
        );
        let (bytes, content_type) =
            serve_cbz_image(cbz_path, "subdir_1/3.jpg", -1)
                .expect("serve jpeg should succeed");

        assert_eq!(content_type, "image/jpeg");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn serve_cbz_image_missing_file_returns_err() {
        let result = serve_cbz_image("/nonexistent/path/file.cbz", "image.png", -1);
        assert!(result.is_err(), "should fail for a non-existent CBZ file");
    }

    #[test]
    fn serve_cbz_image_missing_entry_returns_err() {
        let png_bytes = create_small_png();
        let dir = create_test_cbz_with_png(&png_bytes);
        let cbz_path = dir.path().join("test.cbz");

        let result =
            serve_cbz_image(cbz_path.to_str().unwrap(), "nonexistent_entry.png", -1);
        assert!(result.is_err(), "should fail when entry is not in archive");
    }

    #[test]
    fn serve_cbz_image_thumbnail_size_returns_jpeg() {
        // size = 100 (thumbnail path)
        let cbz_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/books/tests/Testing Introduction 01.cbz"
        );
        let (bytes, content_type) =
            serve_cbz_image(cbz_path, "subdir_2/1.png", 100)
                .expect("thumbnail should succeed");

        assert_eq!(content_type, "image/jpeg");
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xD8);
    }

    #[test]
    fn serve_cbz_image_low_quality_for_small_size() {
        // size < 320 should use quality=40; we just verify it succeeds and is valid JPEG.
        let cbz_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/books/tests/Testing Introduction 01.cbz"
        );
        let (bytes, content_type) =
            serve_cbz_image(cbz_path, "subdir_1/1.png", 200)
                .expect("low-quality encode should succeed");

        assert_eq!(content_type, "image/jpeg");
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xD8);
    }
}