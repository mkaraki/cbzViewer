package main

import (
	"html/template"
	"log"
	"net/http"
	"os"
	"path"
)

type ListItem struct {
	Name  string
	Path  string
	IsDir bool
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
		listData.Items = append(listData.Items, ListItem{
			IsDir: e.IsDir(),
			Name:  e.Name(),
			Path:  path.Join(queryPath, e.Name()),
		})
	}

	err = html.Execute(w, listData)
	if err != nil {
		w.WriteHeader(500)
		log.Println(err)
		return
	}
}
