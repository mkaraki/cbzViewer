package main

import (
	"archive/zip"
	"io"
	"log"
	"net/http"
	"os"
	"strconv"

	"github.com/getsentry/sentry-go"
	"github.com/mkaraki/cbzViewer/lepton_jpeg"
	"gopkg.in/gographics/imagick.v3/imagick"
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
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			span.Finish()
			return
		}

		span_open_zip_img := span.StartChild("open_zip_img")

		imgData, err := zipReader.Open(queryFile)
		if os.IsNotExist(err) {
			w.WriteHeader(404)
			_, _ = w.Write([]byte("No such image"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			span_open_zip_img.Finish()
			span.Finish()
			return
		} else if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to read image file"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			span_open_zip_img.Finish()
			span.Finish()
			return
		}

		span_open_zip_img.Finish()
		span.Finish()

		w.Header().Set("Content-Type", contentType)
		fileCacheSend(checkAbsPath, w)
		sendCacheControl(w)

		if requestExtension == "lep" {
			span_lepton := sentry.StartSpan(ctx, "lepton_jpeg_decode")
			err = lepton_jpeg.DecodeLepton(w, imgData)
			span_lepton.Finish()
		} else {
			_, err = io.Copy(w, imgData)
		}

		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to export image"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			return
		}
	case "pdf":
		span := sentry.StartSpan(ctx, "get_pdf_img")

		span_initialize := span.StartChild("init_pdf_read")
		imagick.Initialize()
		defer imagick.Terminate()
		mw := imagick.NewMagickWand()
		defer mw.Destroy()
		span_initialize.Finish()

		pageNum, err := strconv.Atoi(queryFile)
		if err != nil {
			w.WriteHeader(400)
			_, _ = w.Write([]byte("Unable to get page number"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			return
		}

		if isThumb {
			err = mw.SetResolution(50, 50)
		} else {
			err = mw.SetResolution(350, 350)
		}
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when setting resolution"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			return
		}

		span_read_img := span.StartChild("read_pdf_img")

		err = mw.ReadImage(checkAbsPath + "[" + strconv.Itoa(pageNum-1) + "]")
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when loading pdf file"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			span_read_img.Finish()
			return
		}

		span_read_img.Finish()

		span_remove_alpha := span.StartChild("remove_alpha_channel")

		err = mw.SetImageAlphaChannel(imagick.ALPHA_CHANNEL_OPAQUE)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed to remove alpha channel"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			span_remove_alpha.Finish()
			return
		}

		span_remove_alpha.Finish()

		if !isThumb {
			span_resample := span.StartChild("resample_img")

			err = mw.ResampleImage(192.0, 192.0, imagick.FILTER_CUBIC)
			if err != nil {
				w.WriteHeader(500)
				_, _ = w.Write([]byte("Failed to resample image"))
				if conf.SentryDsn != "" {
					sentry.CaptureException(err)
				}
				log.Println(err)
				span_resample.Finish()
				return
			}

			span_resample.Finish()

			err = mw.SetCompressionQuality(80)
			if err != nil {
				w.WriteHeader(500)
				if conf.SentryDsn != "" {
					sentry.CaptureException(err)
				}
				log.Println(err)
				span_resample.Finish()
				return
			}
		} else {
			// Won't resample because load as lower resolution.

			err = mw.SetCompressionQuality(15)
			if err != nil {
				w.WriteHeader(500)
				if conf.SentryDsn != "" {
					sentry.CaptureException(err)
				}
				log.Println(err)
				return
			}
		}

		span_set_image_format := span.StartChild("set_image_format")

		err = mw.SetImageFormat("webp")
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to convert image"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			span_set_image_format.Finish()
			return
		}

		span_set_image_format.Finish()

		span_get_image_blob := span.StartChild("get_image_blob")

		imgRaw, err := mw.GetImageBlob()
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to export image"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			span_get_image_blob.Finish()
			return
		}

		span_get_image_blob.Finish()

		w.Header().Set("Content-Type", "image/webp")
		fileCacheSend(checkAbsPath, w)
		sendCacheControl(w)
		w.WriteHeader(200)
		_, _ = w.Write(imgRaw)
	default:
		w.WriteHeader(400)
		_, _ = w.Write([]byte("Non supported type."))
		return
	}
}
