package routes

import "net/http"

type Controller struct{}

func (Controller) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func NewMux(p *Controller, v Controller) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /address", (&v).Status)
	mux.HandleFunc("GET /deref", (*p).Status)
	mux.HandleFunc("GET /paren", (v).Status)
	return mux
}
