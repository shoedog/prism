package routes

import (
	"net/http"

	"example.org/routes/service"
)

type Controller struct{}

func (Controller) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func NewMux(target service.Handler) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /status", target.Status)
	return mux
}
