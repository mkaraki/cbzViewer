package main

import (
	"fmt"
	"github.com/getsentry/sentry-go"
	sentryhttp "github.com/getsentry/sentry-go/http"
	"io"
	"log"
	"net/http"
	"os"
	"time"
)

var conf *config

func legalHandler(w http.ResponseWriter, r *http.Request) {
	f, err := os.Open("templates/legal.html")
	if err != nil {
		w.WriteHeader(500)
		log.Println(err)
		return
	}

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
	if conf.SentryDsn != "" {
		sentryOptions.Dsn = conf.SentryDsn
	}

	if err := sentry.Init(sentryOptions); err != nil {
		fmt.Printf("Sentry initialization failed: %v\n", err)
	}
	fmt.Println("Sentry initialized")

	sentry.CaptureMessage("Application started. Check this cause due to unexpected reboot or not.")

	defer sentry.Flush(2 * time.Second)

	sentryHandler := sentryhttp.New(sentryhttp.Options{
		Repanic: true,
		Timeout: 10 * time.Second,
	})

	http.HandleFunc("/list", sentryHandler.HandleFunc(listHandler))
	http.HandleFunc("/read", sentryHandler.HandleFunc(readHandler))
	http.HandleFunc("/img", sentryHandler.HandleFunc(imgHandler))
	http.HandleFunc("/thumb", sentryHandler.HandleFunc(thumbHandler))

	http.HandleFunc("/legal", sentryHandler.HandleFunc(legalHandler))
	http.Handle("/assets/", sentryHandler.Handle(http.StripPrefix("/assets/", fs)))

	fmt.Println("Starting server")
	err = http.ListenAndServe(":8080", nil)
	if err != nil {
		sentry.CaptureException(err)
		log.Fatal(err)
	}
}
