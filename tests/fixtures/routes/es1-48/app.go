package routes

import "net/http"

type Controller struct{}

func (Controller) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func NewMux(c Controller) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /status", c.Status)
	return mux
}
