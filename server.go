package main

import (
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/getsentry/sentry-go"
	sentryhttp "github.com/getsentry/sentry-go/http"
)

var conf *config

func legalHandler(w http.ResponseWriter, _ *http.Request) {
	f, err := os.Open("templates/legal.txt")
	if err != nil {
		w.WriteHeader(500)
		log.Println(err)
		return
	}

	w.Header().Set("Content-Type", "text/plain")
	w.WriteHeader(http.StatusOK)
	_, err = io.Copy(w, f)
	if err != nil {
		w.WriteHeader(500)
		log.Println(err)
		return
	}
}

func main() {
	var err error
	conf, err = loadConfig()
	if err != nil {
		log.Fatal(err)
	}

	fs := http.FileServer(http.Dir("templates/assets/"))

	sentryOptions := sentry.ClientOptions{
		EnableTracing:    true,
		TracesSampleRate: 0.1,
	}

	if err := sentry.Init(sentryOptions); err != nil {
		fmt.Printf("Sentry initialization failed: %v\n", err)
	}
	fmt.Println("Sentry initialized")

	defer sentry.Flush(2 * time.Second)

	sentryHandler := sentryhttp.New(sentryhttp.Options{
		Repanic: true,
		Timeout: 10 * time.Second,
	})

	http.HandleFunc("/list", sentryHandler.HandleFunc(listHandler))
	http.HandleFunc("/read", sentryHandler.HandleFunc(readHandler))
	http.HandleFunc("/img", sentryHandler.HandleFunc(imgHandler))
	http.HandleFunc("/thumb", sentryHandler.HandleFunc(thumbHandler))

	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/" {
			http.Redirect(w, r, "/list", http.StatusMovedPermanently)
		} else {
			http.NotFound(w, r)
		}
	})

	http.HandleFunc("/legal", legalHandler)
	http.Handle("/assets/", http.StripPrefix("/assets/", fs))

	fmt.Println("Starting server")
	err = http.ListenAndServe(":8080", nil)
	if err != nil {
		sentry.CaptureException(err)
		log.Fatal(err)
	}
}
