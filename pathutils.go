package main

import (
	"fmt"
	"log"
	"net/http"
	"path"
	"path/filepath"
	"strings"
)

func getRealPath(clientPath string, httpResponse http.ResponseWriter) (bool, string, error) {
	checkDir := path.Join(conf.CbzDir, clientPath)
	checkAbsPath, err := filepath.Abs(checkDir)
	if err != nil {
		if httpResponse != nil {
			httpResponse.WriteHeader(500)
		}
		log.Println(err)
		return true, "", err
	}

	absPath, err := filepath.Abs(conf.CbzDir)
	if err != nil {
		if httpResponse != nil {
			httpResponse.WriteHeader(500)
		}
		log.Println(err)
		return true, "", err
	}

	if !strings.HasPrefix(checkAbsPath, absPath) {
		if httpResponse != nil {
			httpResponse.WriteHeader(500)
		}
		log.Println("User tried to access ", checkDir, " but abs is", absPath)
		return false, checkAbsPath, fmt.Errorf("user tried to access %v", checkDir)
	}

	return true, checkAbsPath, nil
}

func getParentDir(realPath string) (bool, string, error) {
	var hasParent bool
	var parentDir string

	// Get absolute dir of cbz store root for parent detection.
	absPath, err := filepath.Abs(conf.CbzDir)
	if err != nil {
		return false, "", err
	}

	// Detect there are parent directory that user can access or not.
	parentAbsDir := filepath.Dir(realPath)
	if strings.HasPrefix(parentAbsDir, absPath) {
		hasParent = true
		absPathLen := len(absPath)
		parentDir = parentAbsDir[absPathLen:]

		if parentDir == "/" {
			hasParent = false
		}

		return hasParent, parentDir, nil
	}

	return false, "", nil
}

func getExtensionFromFilePath(queryFile string) string {
	requestExtensionAry := strings.Split(queryFile, ".")
	return strings.ToLower(requestExtensionAry[len(requestExtensionAry)-1])
}

func isSupportedImage(requestExtension string) bool {
	switch requestExtension {
	case "png":
		return true
	case "lep":
		return true
	case "jpg", "jpeg":
		return true
	case "gif":
		return true
	case "avif":
		return true
	case "webp":
		return true
	default:
		return false
	}
}

func isSupportedComic(requestExtension string) bool {
	switch requestExtension {
	case "cbz":
		return true
	case "pdf":
		return true
	default:
		return false
	}
}

func getContentTypeFromExtension(requestExtension string) string {
	var contentType string

	switch requestExtension {
	case "png":
		contentType = "image/png"
	case "lep":
		contentType = "image/jpeg"
	case "jpg", "jpeg":
		contentType = "image/jpeg"
	case "gif":
		contentType = "image/gif"
	case "avif":
		contentType = "image/avif"
	case "webp":
		contentType = "image/webp"
	}

	return contentType
}
