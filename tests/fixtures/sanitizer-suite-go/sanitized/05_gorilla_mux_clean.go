package main

import (
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"github.com/gorilla/mux"
)

func handler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	name := vars["file"]
	cleaned := filepath.Clean(name)
	if !strings.HasPrefix(cleaned, "/uploads") {
		return
	}
	_, _ = os.Open(cleaned)
}
