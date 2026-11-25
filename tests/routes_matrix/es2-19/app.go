package routes

import "net/http"

type controller struct{}

func (*controller) Show(w http.ResponseWriter, r *http.Request) { w.WriteHeader(204) }

func NewRouter() *http.ServeMux {
	mux := http.NewServeMux()
	var active = &controller{}
	mux.HandleFunc("GET /inventory", active.Show)
	return mux
}
