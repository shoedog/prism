package routes

import (
	"net/http"

	"github.com/go-chi/chi"
)

func audit(next http.Handler) http.Handler           { return next }
func metrics(w http.ResponseWriter, r *http.Request) { w.WriteHeader(204) }

func NewRouter() *chi.Mux {
	app := chi.NewRouter()
	app.Group(func(inside chi.Router) {
		inside.Use(audit)
		inside.Get("/metrics", metrics)
	})
	return app
}
