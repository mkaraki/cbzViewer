package main

import (
	"fmt"
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

	http.HandleFunc("/list", listHandler)
	http.HandleFunc("/read", readHandler)
	http.HandleFunc("/img", imgHandler)
	http.HandleFunc("/thumb", thumbHandler)

	http.HandleFunc("/legal", legalHandler)

	fs := http.FileServer(http.Dir("templates/assets/"))
	http.Handle("/assets/", http.StripPrefix("/assets/", fs))

	fmt.Println("Starting server")
	err = http.ListenAndServe(":8080", nil)
	if err != nil {
		log.Fatal(err)
	}
}
