package fixture

import "github.com/go-chi/chi"

func NewApp() *chi.Mux {
	r := chi.NewRouter()
	r.Mount("/adm", adm())
	return r
}
