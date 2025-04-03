package main

import (
	"fmt"
	"github.com/getsentry/sentry-go"
	sentryhttp "github.com/getsentry/sentry-go/http"
	"io"
	"log"
	"net/http"
	"os"
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
			Dsn: conf.SentryDsn,
		}); err != nil {
			fmt.Printf("Sentry initialization failed: %v\n", err)
		}

		println("Sentry initialized")

		sentryHandler := sentryhttp.New(sentryhttp.Options{})

		http.HandleFunc("/list", sentryHandler.HandleFunc(listHandler))
		http.HandleFunc("/read", sentryHandler.HandleFunc(readHandler))
		http.HandleFunc("/img", sentryHandler.HandleFunc(imgHandler))
		http.HandleFunc("/thumb", sentryHandler.HandleFunc(thumbHandler))

		http.HandleFunc("/legal", sentryHandler.HandleFunc(legalHandler))
		http.Handle("/assets/", sentryHandler.Handle(http.StripPrefix("/assets/", fs)))
	} else {
		http.HandleFunc("/list", listHandler)
		http.HandleFunc("/read", readHandler)
		http.HandleFunc("/img", imgHandler)
		http.HandleFunc("/thumb", thumbHandler)

		http.HandleFunc("/legal", legalHandler)
		http.Handle("/assets/", http.StripPrefix("/assets/", fs))
	}

	fmt.Println("Starting server")
	err = http.ListenAndServe(":8080", nil)
	if err != nil {
		log.Fatal(err)
	}
}
