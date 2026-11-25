package fixture

import (
	"net/http"

	"github.com/go-chi/chi"
)

func list(w http.ResponseWriter, r *http.Request) {}

func NewApp() *chi.Mux {
	r := chi.NewRouter()
	r.Route("/api", func(r chi.Router) {
		r.Get("/items", list)
	})
	return r
}
