package routes

import "net/http"

func earlier(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusAccepted)
}

func NewApplications() (*http.ServeMux, *http.ServeMux) {
	previous := http.DefaultServeMux
	defer func() { http.DefaultServeMux = previous }()

	before := http.NewServeMux()
	http.DefaultServeMux = before
	http.HandleFunc("GET /status", earlier)
	after := newCurrent()
	return before, after
}
