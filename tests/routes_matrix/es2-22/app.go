package routes

import (
	"net/http"

	"github.com/go-chi/chi"
)

func dispatch(w http.ResponseWriter, r *http.Request) { w.WriteHeader(204) }
func putDraft(w http.ResponseWriter, r *http.Request) { w.WriteHeader(204) }

func NewRouter() *chi.Mux {
	app := chi.NewRouter()
	app.HandleFunc("/dispatch", dispatch)
	app.MethodFunc("PUT", "/drafts/{id}", putDraft)
	return app
}
