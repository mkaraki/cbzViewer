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

	if conf.SentryDsn != "" {
		if err := sentry.Init(sentry.ClientOptions{
			Dsn:              conf.SentryDsn,
			TracesSampleRate: 1.0,
			EnableTracing:    true,
		}); err != nil {
			fmt.Printf("Sentry initialization failed: %v\n", err)
		}

		fmt.Println("Sentry initialized; DSN:", conf.SentryDsn)
	} else {
		if err := sentry.Init(sentry.ClientOptions{
			TracesSampleRate: 1.0,
			EnableTracing:    true,
		}); err != nil {
			fmt.Printf("Sentry initialization failed: %v\n", err)
		}

		fmt.Println("Sentry initialized; DSN: not set")
	}

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
		log.Fatal(err)
	}
}
