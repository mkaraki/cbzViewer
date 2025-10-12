package main

import (
	"archive/zip"
	"bytes"
	"encoding/xml"
	"html/template"
	"io"
	"log"
	"net/http"
	"strconv"

	"github.com/getsentry/sentry-go"
	"github.com/mattn/natural"
	comicinfo "github.com/mkaraki/go_comic_info"
	"github.com/pdfcpu/pdfcpu/pkg/api"
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
		sentry.CaptureException(err)
		log.Println(err)
		return
	}

	readInfo := ReadInfo{
		Path: queryPath,
	}

	_, readInfo.ParentDir, err = getParentDir(checkAbsPath)
	if err != nil {
		w.WriteHeader(500)
		sentry.CaptureException(err)
		log.Println(err)
		return
	}

	extension := getExtensionFromFilePath(checkAbsPath)

	switch extension {
	case "cbz":
		span := sentry.StartSpan(ctx, "read_cbz_zip_archive")

		zipReader, err := zip.OpenReader(checkAbsPath)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when loading cbz file"))
			sentry.CaptureException(err)
			log.Println(err)
			span.Finish()
			return
		}

		comicInfoSpan := span.StartChild("read_cbz_comic_info")

		comicInfoFile, err := zipReader.Open("ComicInfo.xml")
		if comicInfoFile == nil {
			// If there are no `ComicInfo.xml` file
			// we will just enumerate the files in the zip

			readInfo.Pages, err = getPageListFromCbzEnum(zipReader)
			if err != nil {
				w.WriteHeader(500)
				_, _ = w.Write([]byte("Failed when loading cbz file. Unable to fetch images."))
				sentry.CaptureException(err)
				log.Println(err)
				comicInfoSpan.Finish()
				span.Finish()
				return
			}

			readInfo.PageCnt = len(readInfo.Pages)
			comicInfoSpan.Finish()
		} else if err != nil {
			// On error reading `ComicInfo.xml` file

			w.WriteHeader(500)
			_, _ = w.Write([]byte("Unable to read cbz file"))
			sentry.CaptureException(err)
			log.Println(err)
			comicInfoSpan.Finish()
			span.Finish()
			return
		} else {
			// If `ComicInfo.xml` file exists

			comicInfo := comicinfo.ComicInfo{}

			rawComicInfo := &bytes.Buffer{}
			_, err = io.Copy(rawComicInfo, comicInfoFile)
			if err != nil {
				w.WriteHeader(500)
				_, _ = w.Write([]byte("Failed to read ComicInfo.xml"))
				sentry.CaptureException(err)
				log.Println(err)
				comicInfoSpan.Finish()
				span.Finish()
				return
			}

			err = xml.Unmarshal(rawComicInfo.Bytes(), &comicInfo)
			if err != nil {
				w.WriteHeader(500)
				_, _ = w.Write([]byte("Failed to parse ComicInfo.xml"))
				sentry.CaptureException(err)
				log.Println(err)
				comicInfoSpan.Finish()
				span.Finish()
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
				sentry.CaptureException(err)
				log.Println(err)
				comicInfoSpan.Finish()
				span.Finish()
				return
			}

			readInfo.PageCnt = len(readInfo.Pages)
			comicInfoSpan.Finish()
		}

		span.Finish()
	case "pdf":
		span := sentry.StartSpan(ctx, "read_pdf")
		pageCnt, err := api.PageCountFile(checkAbsPath)
		if err != nil {
			w.WriteHeader(500)
			_, _ = w.Write([]byte("Failed when loading pdf file"))
			sentry.CaptureException(err)
			log.Println(err)
			span.Finish()
			return
		}

		span.Finish()

		readInfo.PageCnt = pageCnt
		for i := 1; i <= pageCnt; i++ {
			readInfo.Pages = append(readInfo.Pages, PageInfo{
				PageNo:    i,
				ImageFile: strconv.Itoa(i),
			})
		}
	default:
		w.WriteHeader(400)
		_, _ = w.Write([]byte("Non supported type."))
		return
	}

	fileCacheSend(checkAbsPath, w)
	sendCacheControl(w)
	err = html.Execute(w, readInfo)
	if err != nil {
		w.WriteHeader(500)
		sentry.CaptureException(err)
		log.Println(err)
		return
	}
}

func getPageListFromCbzEnum(zipReader *zip.ReadCloser) ([]PageInfo, error) {
	var files []string
	for _, e := range zipReader.File {
		files = append(files, e.Name)
	}

	natural.Sort(files)

	var i = 0

	var pageInfo []PageInfo
	for _, f := range files {
		fileExt := getExtensionFromFilePath(f)
		if !isSupportedImage(fileExt) {
			continue
		}

		pageInfo = append(pageInfo, PageInfo{PageNo: i + 1, ImageFile: f})
		i++
	}

	return pageInfo, nil
}
