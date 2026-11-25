package fixture

import (
	"net/http"

	"github.com/go-chi/chi"
)

func roster(w http.ResponseWriter, r *http.Request) {}

func NewApp() *chi.Mux {
	r := chi.NewRouter()
	r.Route("/teams", func(teams chi.Router) {
		teams.Route("/{team}", func(team chi.Router) {
			team.Get("/roster", roster)
		})
	})
	return r
}
