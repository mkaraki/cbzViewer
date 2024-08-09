package main

import (
	"fmt"
	"log"
	"net/http"
)

var conf *config

func main() {
	var err error
	conf, err = loadConfig()
	if err != nil {
		log.Fatal(err)
	}

	http.HandleFunc("/list", listHandler)
	http.HandleFunc("/read", readHandler)
	http.HandleFunc("/img", imgHandler)

	fs := http.FileServer(http.Dir("templates/assets/"))
	http.Handle("/assets/", http.StripPrefix("/assets/", fs))

	fmt.Println("Starting server")
	err = http.ListenAndServe(":8080", nil)
	if err != nil {
		log.Fatal(err)
	}
}
