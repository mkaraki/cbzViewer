package main

import (
	"archive/zip"
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

	// Check is user accessible and what dir/file user want to access.
	isUserAccessible, checkAbsPath, err := getRealPath(queryPath, w)

	if !isUserAccessible || err != nil {
		// HTTP response is already returned by getRealPath
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
			log.Println(err)
			return
		}

		imgData, err := zipReader.Open(queryFile)
		if os.IsNotExist(err) {
			w.WriteHeader(404)
			_, _ = w.Write([]byte("No such image"))
			return
		} else if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to read image file"))
			return
		}

		w.Header().Set("Content-Type", contentType)

		if requestExtension == "lep" {
			err = decodeLepton(w, imgData)
		} else {
			_, err = io.Copy(w, imgData)
		}
		if err != nil {
			w.WriteHeader(500)
			log.Println(err)
			return
		}
	case "pdf":
		imagick.Initialize()
		defer imagick.Terminate()
		mw := imagick.NewMagickWand()
		defer mw.Destroy()
		err = mw.ReadImage(checkAbsPath)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when loading pdf file"))
			log.Println(err)
			return
		}

		pageNum, err := strconv.Atoi(queryFile)
		if err != nil {
			w.WriteHeader(400)
			_, _ = w.Write([]byte("Unable to get page number"))
			log.Println(err)
			return
		}

		mw.SetIteratorIndex(pageNum - 1 /* Request starts with 1 but imagick takes start with 0	 */)

		err = mw.SetImageFormat("jpg")
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to convert image"))
			log.Println(err)
			return
		}

		img_raw, err := mw.GetImageBlob()
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to export image"))
			log.Println(err)
			return
		}

		w.Header().Set("Content-Type", "image/jpeg")
		w.WriteHeader(200)
		_, _ = w.Write(img_raw)
	default:
		w.WriteHeader(400)
		_, _ = w.Write([]byte("Non supported type."))
		return
	}
}
