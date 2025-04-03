package main

import (
	"encoding/json"
	"log"
	"os"
)

type config struct {
	CbzDir    string `json:"cbzDir"`
	SentryDsn string `json:"sentryDsn"`
}

func loadConfig() (*config, error) {
	f, err := os.Open("config.json")
	if err != nil {
		log.Fatal(err)
		return nil, err
	}
	defer f.Close()

	var conf config
	err = json.NewDecoder(f).Decode(&conf)
	return &conf, err
}
