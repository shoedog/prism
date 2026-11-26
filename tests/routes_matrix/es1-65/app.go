package routes

import (
	"net/http"

	"example.org/routes/endpoint"
)

type Handler struct{}

func (*Handler) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusAccepted)
}

func NewMux() *http.ServeMux {
	mux := http.NewServeMux()
	target := endpoint.MakeHandler()
	mux.HandleFunc("GET /status", target.Status)
	return mux
}
