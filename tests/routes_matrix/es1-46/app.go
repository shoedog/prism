package routes

import "net/http"

type endpoint interface {
	Status(http.ResponseWriter, *http.Request)
}

type ready struct{}

func (ready) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

type busy struct{}

func (busy) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusAccepted)
}

func NewMux() *http.ServeMux {
	mux := http.NewServeMux()
	mount(mux, "/ready", ready{})
	mount(mux, "/busy", busy{})
	return mux
}

func mount[T endpoint](mux *http.ServeMux, path string, handler T) {
	mux.HandleFunc(path, handler.Status)
}
