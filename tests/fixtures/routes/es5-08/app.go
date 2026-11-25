package routes

import "net/http"

type source struct{}

func (*source) Status(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusOK) }

type sourceAlias = source
type independent source

func (*independent) Status(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusAccepted)
}

func NewMux(original *sourceAlias, replacement *independent) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /source", original.Status)
	mux.HandleFunc("GET /replacement", replacement.Status)
	return mux
}
