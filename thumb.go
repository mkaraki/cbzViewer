package main

import (
	"archive/zip"
	"io/fs"
	"log"
	"net/http"
	"net/url"
	"path"
	"path/filepath"

	"github.com/getsentry/sentry-go"
)

func thumbHandler(w http.ResponseWriter, r *http.Request) {
	// Get query params
	query := r.URL.Query()

	// If there are no `path` query. add `/` for it.
	if !query.Has("path") {
		w.WriteHeader(400)
		return
	}

	queryPath := query.Get("path")

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

	firstPageName := getFirstPageName(checkAbsPath)

	if firstPageName == "" {
		w.WriteHeader(404)
		return
	}

	imgLocation := "img?path=" + url.QueryEscape(queryPath) +
		"&f=" + url.QueryEscape(firstPageName) +
		"&thumb=1"

	fileCacheSend(checkAbsPath, w)
	sendCacheControl(w)
	w.Header().Set("Location", imgLocation)
	w.WriteHeader(301)
}

func getFirstPageName(comicFilePath string) string {
	extension := getExtensionFromFilePath(comicFilePath)

	switch extension {
	case "cbz":
		zipReader, err := zip.OpenReader(comicFilePath)
		if err != nil {
			log.Println(err)
			return ""
		}

		pages, err := getPageListFromCbzEnum(zipReader)
		if err != nil {
			log.Println(err)
			return ""
		}

		if len(pages) < 1 {
			log.Println("no pages exists")
			return ""
		}

		return pages[0].ImageFile
	case "pdf":
		return "1"
	default:
		log.Println("unknown comic format")
		return ""
	}
}

func dirThumbHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	hub := sentry.GetHubFromContext(ctx)
	if hub == nil {
		hub = sentry.CurrentHub().Clone()
		ctx = sentry.SetHubOnContext(ctx, hub)
	}

	// Get query params
	query := r.URL.Query()

	// If there are no `path` query. add `/` for it.
	if !query.Has("path") {
		query.Set("path", "/")
	}

	// read `path` params
	queryPath := query.Get("path")

	// Check is user accessible and what dir/file user want to access.
	isUserAccessible, checkAbsPath, err := getRealPath(queryPath, w)

	if !isUserAccessible || err != nil {
		// HTTP response is already returned by getRealPath
		return
	}

	// Get files in directory
	thumbPath := ""
	span := sentry.StartSpan(ctx, "dir.walk")
	span.Name = "Walk directory to get item for thumb"
	span.SetTag("path", checkAbsPath)
	err = filepath.WalkDir(checkAbsPath, func(p string, info fs.DirEntry, err error) error {
		if err != nil {
			sentry.CaptureException(err)
			log.Println(err)
			return err
		}

		if thumbPath != "" {
			return filepath.SkipDir
		}

		if info.IsDir() {
			return nil
		}

		fileExt := getExtensionFromFilePath(info.Name())
		if isSupportedComic(fileExt) {
			p = p[len(checkAbsPath):]
			thumbPath = path.Join(queryPath, p)
			return filepath.SkipDir
		}

		return nil
	})
	span.Finish()

	thumbLocation := "thumb?path=" + url.QueryEscape(thumbPath)

	fileCacheSend(checkAbsPath, w)
	sendCacheControl(w)
	w.Header().Set("Location", thumbLocation)
	w.WriteHeader(301)
}
