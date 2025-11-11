package main

import (
	"encoding/json"
	"log"
	"net/http"
	"os"
	"path"

	"github.com/getsentry/sentry-go"
)

type ListItem struct {
	Name  string `json:"name"`
	Path  string `json:"path"`
	IsDir bool   `json:"isDir"`
}

type ListData struct {
	Items      []ListItem `json:"items"`
	CurrentDir string     `json:"currentDir"`
	HasParent  bool       `json:"hasParent"`
	ParentDir  string     `json:"parentDir"`
}

func listApiHandler(w http.ResponseWriter, r *http.Request) {
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
	span := sentry.StartSpan(ctx, "read_dir")
	entries, err := os.ReadDir(checkAbsPath)
	span.Finish()

	if os.IsNotExist(err) {
		// Is not directory or directory not exists
		w.WriteHeader(404)
		return
	} else if err != nil {
		// Unknown error
		w.WriteHeader(500)
		sentry.CaptureException(err)
		log.Println(err)
		return
	}

	listData := ListData{
		CurrentDir: queryPath,
		HasParent:  false,
		ParentDir:  "",
	}

	listData.HasParent, listData.ParentDir, err = getParentDir(checkAbsPath)
	if err != nil {
		w.WriteHeader(500)
		sentry.CaptureException(err)
		log.Println(err)
		return
	}

	for _, e := range entries {
		ext := getExtensionFromFilePath(e.Name())
		isDir := e.IsDir()

		fileName := e.Name()
		filePath := path.Join(queryPath, fileName)

		if !isDir && !isSupportedComic(ext) {
			continue
		}

		listItem := ListItem{
			IsDir: isDir,
			Name:  fileName,
			Path:  filePath,
		}

		listData.Items = append(listData.Items, listItem)
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(200)
	err = json.NewEncoder(w).Encode(listData)
	if err != nil {
		println(err)
		sentry.CaptureException(err)
	}
}
