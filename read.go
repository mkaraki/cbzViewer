package main

import (
	"archive/zip"
	"html/template"
	"log"
	"net/http"
	"sort"
	"strconv"
)

type PageInfo struct {
	PageNo    int
	ImageFile string
}

type ReadInfo struct {
	ComicTitle string
	Pages      []PageInfo
	Path       string
	PageCnt    int
	ParentDir  string
}

func readHandler(w http.ResponseWriter, r *http.Request) {
	// Get query params
	query := r.URL.Query()

	// If there are no `path` query. add `/` for it.
	if !query.Has("path") {
		w.WriteHeader(400)
		return
	}

	queryPath := query.Get("path")

	// Read template
	html, err := template.ParseFiles("templates/read.html")
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

	readInfo := ReadInfo{
		Path: queryPath,
	}

	_, readInfo.ParentDir, err = getParentDir(checkAbsPath)
	if err != nil {
		w.WriteHeader(500)
		log.Println(err)
		return
	}

	extension := getExtensionFromFilePath(checkAbsPath)

	switch extension {
	case "cbz":
		zipReader, err := zip.OpenReader(checkAbsPath)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when loading cbz file"))
			log.Println(err)
			return
		}

		/*comicInfoFile, err := zipReader.Open("ComicInfo.xml")
		if os.IsNotExist(err) {
			readInfo.Pages, err = getPageListFromCbzEnum(zipReader)
			if err != nil {
				w.WriteHeader(500)
				w.Write([]byte("Failed when loading cbz file. Unable to fetch images."))
				log.Println(err)
				return
			}

			readInfo.PageCnt = len(readInfo.Pages)
		} else if err != nil {
			w.WriteHeader(500)
			w.Write([]byte("Unable to read cbz file"))
			return
		}*/

		//comicInfoFile.Read()
		readInfo.Pages, err = getPageListFromCbzEnum(zipReader)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when loading cbz file. Unable to fetch images."))
			log.Println(err)
			return
		}

		readInfo.PageCnt = len(readInfo.Pages)
	default:
		w.WriteHeader(400)
		_, _ = w.Write([]byte("Non supported type."))
		return
	}

	err = html.Execute(w, readInfo)
	if err != nil {
		w.WriteHeader(500)
		log.Println(err)
		return
	}
}

func getPageListFromCbzEnum(zipReader *zip.ReadCloser) ([]PageInfo, error) {
	var files []string
	for _, e := range zipReader.File {
		files = append(files, e.Name)
	}

	sort.Slice(files, func(i, j int) bool {
		ni, _ := strconv.Atoi(files[i])
		nj, _ := strconv.Atoi(files[j])
		return ni < nj
	})

	var pageInfo []PageInfo
	for i, f := range files {
		fileExt := getExtensionFromFilePath(f)
		if !isSupportedImage(fileExt) {
			continue
		}

		pageInfo = append(pageInfo, PageInfo{PageNo: i + 1, ImageFile: f})
	}

	return pageInfo, nil
}
