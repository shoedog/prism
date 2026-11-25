package routes

import (
	"net/http"

	"github.com/go-chi/chi"
)

func files(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func NewApp() *chi.Mux {
	root := chi.NewRouter()
	root.Route("/{account:[a-z]+}", func(accountRoutes chi.Router) {
		accountRoutes.Get("/files/*", files)
	})
	return root
}
