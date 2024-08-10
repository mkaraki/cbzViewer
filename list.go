package main

import (
	"archive/zip"
	"html/template"
	"log"
	"net/http"
	"os"
	"path"
)

type ListItem struct {
	Name    string
	Path    string
	IsDir   bool
	TopPage PageInfo
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

		var pageInfo PageInfo

		if !isDir && !isSupportedComic(ext) {
			continue
		} else if !isDir {
			pageInfo = PageInfo{
				PageNo:    0,
				ImageFile: getFirstPageName(filePath),
			}
		}

		listData.Items = append(listData.Items, ListItem{
			IsDir:   isDir,
			Name:    fileName,
			Path:    filePath,
			TopPage: pageInfo,
		})
	}

	err = html.Execute(w, listData)
	if err != nil {
		w.WriteHeader(500)
		log.Println(err)
		return
	}
}

func getFirstPageName(comicFilePath string) string {
	extension := getExtensionFromFilePath(comicFilePath)
	_, checkAbsPath, err := getRealPath(comicFilePath, nil)
	if err != nil {
		log.Println(err)
		return ""
	}

	switch extension {
	case "cbz":
		zipReader, err := zip.OpenReader(checkAbsPath)
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
	default:
		log.Println("unknown comic format")
		return ""
	}
}
