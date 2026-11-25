package routes

import "net/http"

type renderer[T any] struct{}

func (renderer[T]) Render(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

type detailView struct {
	renderer[string]
}

func NewApp() *http.ServeMux {
	mux := http.NewServeMux()
	view := detailView{}
	mux.HandleFunc("GET /embedded", view.Render)
	mux.HandleFunc("GET /explicit", view.renderer.Render)
	return mux
}
