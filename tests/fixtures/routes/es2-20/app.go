package routes

import "net/http"

func batches(w http.ResponseWriter, r *http.Request) { w.WriteHeader(204) }
func active(w http.ResponseWriter, r *http.Request)  { w.WriteHeader(200) }

func Register() *http.ServeMux {
	http.DefaultServeMux.HandleFunc("GET /batches", batches)
	http.HandleFunc("GET /active", active)
	return http.DefaultServeMux
}
