package routes

import (
	"net/http"

	"github.com/go-chi/chi"
)

func ready(w http.ResponseWriter, _ *http.Request)    { w.WriteHeader(http.StatusOK) }
func download(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusAccepted) }

func App() *chi.Mux {
	r := chi.NewRouter()
	r.Get("/ready", ready)
	r.Mount("/downloads", http.HandlerFunc(download))
	return r
}
