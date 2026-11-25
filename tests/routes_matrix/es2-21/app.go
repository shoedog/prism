package routes

import "net/http"

func item(w http.ResponseWriter, r *http.Request) { w.WriteHeader(204) }

func NewRouter() *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc(`PUT	warehouse.example.test/items/%7Bdraft%7D/{id}`, item)
	return mux
}
