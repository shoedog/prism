package routes

import "net/http"

type Base struct{}

func (*Base) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

type wrapper struct{ *Base }

func NewMux(w wrapper) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /status", w.Status)
	return mux
}
