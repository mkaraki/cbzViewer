package main

import (
	"fmt"
	"html/template"
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
	f, err := os.Open("dist/legal.txt")
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

type vueInitialData struct {
	SentryBaggage string
	SentryTrace   string
	SentryDsn     string
	ServerHost    string
}

func main() {
	var err error
	conf, err = loadConfig()
	if err != nil {
		log.Fatal(err)
	}

	fs := http.FileServer(http.Dir("dist/assets/"))

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

	http.HandleFunc("/api/list", sentryHandler.HandleFunc(listApiHandler))
	http.HandleFunc("/api/read", sentryHandler.HandleFunc(readApiHandler))
	http.HandleFunc("/api/img", sentryHandler.HandleFunc(imgHandler))
	http.HandleFunc("/api/thumb", sentryHandler.HandleFunc(thumbHandler))
	http.HandleFunc("/api/thumb_dir", sentryHandler.HandleFunc(dirThumbHandler))

	http.HandleFunc("/", sentryHandler.HandleFunc(func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()
		hub := sentry.GetHubFromContext(ctx)
		if hub == nil {
			hub = sentry.CurrentHub().Clone()
			ctx = sentry.SetHubOnContext(ctx, hub)
		}

		if r.URL.Path == "/favicon.ico" || r.URL.Path == "/robots.txt" {
			http.NotFound(w, r)
		} else {
			html, err := template.ParseFiles("dist/index.html")
			if err != nil {
				sentry.CaptureException(err)
				log.Println(err)
				w.WriteHeader(500)
				_, err = w.Write([]byte("Couldn't prepare frontend HTML."))
				if err != nil {
					log.Println(err)
					return
				}
				return
			}
			data := vueInitialData{
				SentryBaggage: hub.GetBaggage(),
				SentryTrace:   hub.GetTraceparent(),
				ServerHost:    r.Host,
				SentryDsn:     os.Getenv("SENTRY_DSN"),
			}

			err = html.Execute(w, data)
			if err != nil {
				w.WriteHeader(500)
				sentry.CaptureException(err)
				log.Println(err)
			}
		}
	}))

	http.HandleFunc("/legal", legalHandler)
	http.Handle("/assets/", http.StripPrefix("/assets/", fs))

	fmt.Println("Starting server")
	err = http.ListenAndServe(":8080", nil)
	if err != nil {
		sentry.CaptureException(err)
		log.Fatal(err)
	}
}
