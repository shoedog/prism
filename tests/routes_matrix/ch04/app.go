package fixture

import (
	"net/http"

	"github.com/go-chi/chi"
)

func mw(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		next.ServeHTTP(w, r)
	})
}

func list(w http.ResponseWriter, r *http.Request) {}

func NewApp() *chi.Mux {
	r := chi.NewRouter()
	r.With(mw).Get("/items", list)
	return r
}
