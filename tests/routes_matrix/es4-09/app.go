package routes

import "net/http"

type viewer interface {
	Show(http.ResponseWriter, *http.Request)
}

type report struct{}

func (*report) Show(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func NewApp() *http.ServeMux {
	mux := http.NewServeMux()
	var selected viewer = &report{}
	var current *report = &report{}
	mux.HandleFunc("GET /interface", selected.Show)
	mux.HandleFunc("GET /direct", current.Show)
	return mux
}
