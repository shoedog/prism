package fixture

import (
	"net/http"

	"github.com/go-chi/chi"
)

func audit(w http.ResponseWriter, r *http.Request) {}

func adm() *chi.Mux {
	admin := chi.NewRouter()
	admin.Get("/audit", audit)
	return admin
}
