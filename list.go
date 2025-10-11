package main

import (
	"github.com/getsentry/sentry-go"
	"html/template"
	"io/fs"
	"log"
	"net/http"
	"os"
	"path"
	"path/filepath"
)

type ListItem struct {
	Name      string
	Path      string
	IsDir     bool
	ThumbPath string
}

type ListData struct {
	Items      []ListItem
	CurrentDir string
	HasParent  bool
	ParentDir  string
}

func listHandler(w http.ResponseWriter, r *http.Request) {
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

	// Read template
	html, err := template.ParseFiles("templates/list.html")
	if err != nil {
		w.WriteHeader(500)
		sentry.CaptureException(err)
		log.Println(err)
		return
	}

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
		realFilePath := path.Join(checkAbsPath, fileName)

		if !isDir && !isSupportedComic(ext) {
			continue
		}

		listItem := ListItem{
			IsDir:     isDir,
			Name:      fileName,
			Path:      filePath,
			ThumbPath: "",
		}

		if isDir {
			span := sentry.StartSpan(ctx, "walk_child_comic_for_directory_thumb")
			span.SetTag("path", filePath)
			err = filepath.WalkDir(realFilePath, func(p string, info fs.DirEntry, err error) error {
				if err != nil {
					sentry.CaptureException(err)
					log.Println(err)
					return err
				}

				if listItem.ThumbPath != "" {
					return filepath.SkipDir
				}

				if info.IsDir() {
					return nil
				}

				fileExt := getExtensionFromFilePath(info.Name())
				if isSupportedComic(fileExt) {
					p = p[len(realFilePath):]
					listItem.ThumbPath = path.Join(filePath, p)
					return filepath.SkipDir
				}

				return nil
			})
			span.Finish()
		}

		listData.Items = append(listData.Items, listItem)
	}

	err = html.Execute(w, listData)
	if err != nil {
		w.WriteHeader(500)
		sentry.CaptureException(err)
		log.Println(err)
		return
	}
}
