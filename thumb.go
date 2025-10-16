package main

import (
	"archive/zip"
	"log"
	"net/http"
	"net/url"

	"github.com/getsentry/sentry-go"
)

func thumbHandler(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	hub := sentry.GetHubFromContext(ctx)
	sentry.ContinueTrace(hub, r.Header.Get(sentry.SentryTraceHeader), r.Header.Get(sentry.SentryBaggageHeader))

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
