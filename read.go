package main

import (
	"archive/zip"
	"bytes"
	"encoding/xml"
	"github.com/mkaraki/cbzViewer/comic_info"
	"gopkg.in/gographics/imagick.v2/imagick"
	"html/template"
	"io"
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

	// Read template
	html, err := template.ParseFiles("templates/read.html")
	if err != nil {
		w.WriteHeader(500)
		log.Println(err)
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

		comicInfoFile, err := zipReader.Open("ComicInfo.xml")
		if comicInfoFile == nil {
			readInfo.Pages, err = getPageListFromCbzEnum(zipReader)
			if err != nil {
				w.WriteHeader(500)
				_, _ = w.Write([]byte("Failed when loading cbz file. Unable to fetch images."))
				log.Println(err)
				return
			}

			readInfo.PageCnt = len(readInfo.Pages)
		} else if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to read cbz file"))
			return
		} else {
			comicInfo := comic_info.ComicInfo{}

			rawComicInfo := &bytes.Buffer{}
			_, err = io.Copy(rawComicInfo, comicInfoFile)
			if err != nil {
				w.WriteHeader(500)
				_, _ = w.Write([]byte("Failed to read ComicInfo.xml"))
				log.Println(err)
				return
			}

			err = xml.Unmarshal(rawComicInfo.Bytes(), &comicInfo)
			if err != nil {
				w.WriteHeader(500)
				_, _ = w.Write([]byte("Failed to parse ComicInfo.xml"))
				log.Println(err)
				return
			}

			readInfo.ComicTitle = comicInfo.Title
			if comicInfo.Series != "" {
				readInfo.ComicTitle += " - " + comicInfo.Series
			}

			readInfo.Pages, err = getPageListFromCbzEnum(zipReader)
			if err != nil {
				w.WriteHeader(500)
				_, _ = w.Write([]byte("Failed when loading cbz file. Unable to fetch images."))
				log.Println(err)
				return
			}

			readInfo.PageCnt = len(readInfo.Pages)
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
		pageCnt := mw.GetNumberImages()
		readInfo.PageCnt = int(pageCnt)
		for i := uint(1); i <= pageCnt; i++ {
			readInfo.Pages = append(readInfo.Pages, PageInfo{
				PageNo:    int(i),
				ImageFile: strconv.Itoa(int(i)),
			})
		}
	default:
		w.WriteHeader(400)
		_, _ = w.Write([]byte("Non supported type."))
		return
	}

	fileCacheSend(checkAbsPath, w)
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
