package main

import (
	"html/template"
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
	entries, err := os.ReadDir(checkAbsPath)

	if os.IsNotExist(err) {
		// Is not directory or directory not exists
		w.WriteHeader(404)
		return
	} else if err != nil {
		// Unknown error
		w.WriteHeader(500)
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
			err = filepath.Walk(realFilePath, func(p string, info os.FileInfo, err error) error {
				if listItem.ThumbPath != "" {
					return nil
				}

				if info.IsDir() {
					return nil
				}

				fileExt := getExtensionFromFilePath(info.Name())
				if isSupportedComic(fileExt) {
					p = p[len(realFilePath):]
					listItem.ThumbPath = path.Join(filePath, p)
				}

				return nil
			})
		}

		listData.Items = append(listData.Items, listItem)
	}

	err = html.Execute(w, listData)
	if err != nil {
		w.WriteHeader(500)
		log.Println(err)
		return
	}
}
