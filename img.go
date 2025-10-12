package main

import (
	"archive/zip"
	"bytes"
	"io"
	"log"
	"net/http"
	"os"
	"strconv"

	"github.com/davidbyttow/govips/v2/vips"
	"github.com/getsentry/sentry-go"
	"github.com/mkaraki/cbzViewer/lepton_jpeg"
)

func imgHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	hub := sentry.GetHubFromContext(ctx)
	if hub == nil {
		hub = sentry.CurrentHub().Clone()
		ctx = sentry.SetHubOnContext(ctx, hub)
	}

	// Get query params
	query := r.URL.Query()

	// If there are no `path` query. add `/` for it.
	if !query.Has("path") || !query.Has("f") {
		w.WriteHeader(400)
		return
	}

	queryPath := query.Get("path")
	queryFile := query.Get("f")

	isThumb := query.Has("thumb")
	size := -1
	if isThumb {
		size = 100
	}
	if query.Has("size") {
		size = int(query.Get("size")[0])
	}

	// Check is user accessible and what dir/file user want to access.
	isUserAccessible, checkAbsPath, err := getRealPath(queryPath, w)

	if !isUserAccessible || err != nil {
		// HTTP response is already returned by getRealPath
		return
	}

	cacheActive := fileCacheCheck(checkAbsPath, w, r)
	if cacheActive {
		return
	}

	baseFileExtension := getExtensionFromFilePath(checkAbsPath)

	switch baseFileExtension {
	case "cbz":
		requestExtension := getExtensionFromFilePath(queryFile)
		contentType := getContentTypeFromExtension(requestExtension)

		if !isSupportedImage(requestExtension) {
			w.WriteHeader(400)
			_, _ = w.Write([]byte("Not a supported image"))
			return
		}

		span := sentry.StartSpan(ctx, "open_zip")

		zipReader, err := zip.OpenReader(checkAbsPath)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when loading cbz file"))
			sentry.CaptureException(err)
			log.Println(err)
			span.Finish()
			return
		}

		spanOpenZipImg := span.StartChild("open_zip_img")

		imgData, err := zipReader.Open(queryFile)
		if os.IsNotExist(err) {
			w.WriteHeader(404)
			_, _ = w.Write([]byte("No such image"))
			sentry.CaptureException(err)
			spanOpenZipImg.Finish()
			span.Finish()
			return
		} else if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to read image file"))
			sentry.CaptureException(err)
			log.Println(err)
			spanOpenZipImg.Finish()
			span.Finish()
			return
		}

		spanOpenZipImg.Finish()
		span.Finish()

		if size == -1 {
			// If original
			w.Header().Set("Content-Type", contentType)
		} else {
			// If resizable
			w.Header().Set("Content-Type", "image/webp")
		}
		fileCacheSend(checkAbsPath, w)
		sendCacheControl(w)

		imgBinary := &bytes.Buffer{}

		if requestExtension == "lep" {
			spanLepton := sentry.StartSpan(ctx, "lepton_jpeg_decode")
			if size == -1 { // Original
				err = lepton_jpeg.DecodeLepton(w, imgData)
			} else {
				err = lepton_jpeg.DecodeLepton(imgBinary, imgData)
			}
			spanLepton.Finish()
		} else {
			if size == -1 { // Original
				_, err = io.Copy(w, imgData)
			} else {
				_, err = io.Copy(imgBinary, imgData)
			}
		}

		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to export image"))
			sentry.CaptureException(err)
			log.Println(err)
			return
		}

		if size == -1 {
			// If original, image data already sent.
			return
		}

		// =================================
		// Resize
		// =================================

		err = imgData.Close()
		if err != nil {
			// This is not fatal error.
			sentry.CaptureException(err)
		}

		imgObject, err := vips.NewImageFromReader(imgBinary)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to decode image"))
			sentry.CaptureException(err)
		}
		imgBinary.Reset() // Clear memory.

		imgResizeRate := float64(size) / float64(imgObject.Width())
		if imgResizeRate < 1.0 {
			err = imgObject.Resize(imgResizeRate, vips.KernelLanczos3)
			if err != nil {
				// This is continuable error.
				sentry.CaptureException(err)
				log.Println(err)
				return
			}
		}

		webpParams := vips.NewWebpExportParams()
		webpParams.StripMetadata = true
		if size < 320 {
			webpParams.Quality = 20
		} else {
			webpParams.Quality = 90
		}

		imgBytes, _, err := imgObject.ExportWebp(webpParams)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to export image"))
			sentry.CaptureException(err)
			log.Println(err)
			return
		}

		w.WriteHeader(200)
		_, err = w.Write(imgBytes)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to export image"))
			sentry.CaptureException(err)
		}
	case "pdf":
		span := sentry.StartSpan(ctx, "get_pdf_img")

		pageNum, err := strconv.Atoi(queryFile)
		if err != nil {
			w.WriteHeader(400)
			_, _ = w.Write([]byte("Unable to get page number"))
			sentry.CaptureException(err)
			log.Println(err)
			return
		}

		importParam := vips.NewImportParams()
		importParam.Page.Set(pageNum - 1)

		webpParam := vips.NewWebpExportParams()
		webpParam.StripMetadata = true

		if isThumb {
			importParam.Density.Set(50)
			webpParam.Quality = 20
		} else {
			importParam.Density.Set(350)
			webpParam.Quality = 90
		}

		spanReadImg := span.StartChild("read_pdf_img")

		image, err := vips.LoadImageFromFile(checkAbsPath, importParam)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when loading pdf file"))
			sentry.CaptureException(err)
			log.Println(err)
			spanReadImg.Finish()
			return
		}

		spanReadImg.Finish()

		imgBytes, _, err := image.ExportWebp(webpParam)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when exporting pdf file"))
			sentry.CaptureException(err)
			log.Println(err)
		}

		w.Header().Set("Content-Type", "image/webp")
		fileCacheSend(checkAbsPath, w)
		sendCacheControl(w)
		w.WriteHeader(200)
		_, err = w.Write(imgBytes)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when exporting pdf file"))
			sentry.CaptureException(err)
			log.Println(err)
		}
	default:
		w.WriteHeader(400)
		_, _ = w.Write([]byte("Non supported type."))
		return
	}
}
