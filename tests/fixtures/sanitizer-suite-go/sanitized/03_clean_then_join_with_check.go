package main

import (
	"net/http"
	"path/filepath"
	"strings"
)

func handler(w http.ResponseWriter, r *http.Request) {
	name := r.URL.Query().Get("name")
	cleaned := filepath.Clean(name)
	if !strings.HasPrefix(cleaned, "/static") {
		http.Error(w, "forbidden", http.StatusForbidden)
		return
	}
	http.ServeFile(w, r, cleaned)
}
