package routes

import (
	"net/http"

	"github.com/go-chi/chi"
)

func properties(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusOK) }
func list(w http.ResponseWriter, _ *http.Request)       { w.WriteHeader(http.StatusNoContent) }

func NewApp() *chi.Mux {
	chi.RegisterMethod("PROPFIND")
	app := chi.NewRouter()
	app.MethodFunc("PROPFIND", "/documents", properties)
	app.Get("/documents", list)
	return app
}
