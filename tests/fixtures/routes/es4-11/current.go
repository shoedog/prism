package routes

import "net/http"

func current(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func newCurrent() *http.ServeMux {
	after := http.NewServeMux()
	http.DefaultServeMux = after
	http.HandleFunc("GET /status", current)
	return after
}
