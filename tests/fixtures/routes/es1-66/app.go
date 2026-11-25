package routes

import "net/http"

func items(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func NewMux() *http.ServeMux {
	child := http.NewServeMux()
	child.HandleFunc("GET /api/items", items)
	app := http.NewServeMux()
	app.Handle("/api/", child)
	return app
}
