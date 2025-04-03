package main

import (
	"archive/zip"
	"github.com/getsentry/sentry-go"
	"github.com/mkaraki/cbzViewer/lepton_jpeg"
	"gopkg.in/gographics/imagick.v2/imagick"
	"io"
	"log"
	"net/http"
	"os"
	"strconv"
)

func imgHandler(w http.ResponseWriter, r *http.Request) {
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

		zipReader, err := zip.OpenReader(checkAbsPath)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when loading cbz file"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			return
		}

		imgData, err := zipReader.Open(queryFile)
		if os.IsNotExist(err) {
			w.WriteHeader(404)
			_, _ = w.Write([]byte("No such image"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			return
		} else if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to read image file"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			return
		}

		w.Header().Set("Content-Type", contentType)
		fileCacheSend(checkAbsPath, w)

		if requestExtension == "lep" {
			err = lepton_jpeg.DecodeLepton(w, imgData)
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
		imagick.Initialize()
		defer imagick.Terminate()
		mw := imagick.NewMagickWand()
		defer mw.Destroy()

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

		err = mw.ReadImage(checkAbsPath + "[" + strconv.Itoa(pageNum-1) + "]")
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when loading pdf file"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			return
		}

		err = mw.SetImageAlphaChannel(imagick.ALPHA_CHANNEL_FLATTEN)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed to remove alpha channel"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			return
		}

		if !isThumb {
			err = mw.ResampleImage(192, 192, imagick.FILTER_CUBIC, 1.0)
			if err != nil {
				w.WriteHeader(500)
				_, _ = w.Write([]byte("Failed to resample image"))
				if conf.SentryDsn != "" {
					sentry.CaptureException(err)
				}
				log.Println(err)
				return
			}

			err = mw.SetCompressionQuality(80)
			if err != nil {
				w.WriteHeader(500)
				if conf.SentryDsn != "" {
					sentry.CaptureException(err)
				}
				log.Println(err)
				return
			}
		} else {
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

		err = mw.SetImageFormat("webp")
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to convert image"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			return
		}

		imgRaw, err := mw.GetImageBlob()
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to export image"))
			if conf.SentryDsn != "" {
				sentry.CaptureException(err)
			}
			log.Println(err)
			return
		}

		w.Header().Set("Content-Type", "image/webp")
		fileCacheSend(checkAbsPath, w)
		w.WriteHeader(200)
		_, _ = w.Write(imgRaw)
	default:
		w.WriteHeader(400)
		_, _ = w.Write([]byte("Non supported type."))
		return
	}
}
