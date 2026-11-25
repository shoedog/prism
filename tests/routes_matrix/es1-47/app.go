package routes

import (
	"net/http"

	"example.org/routes/panel"
)

type preview struct{}

func (preview) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func NewMux() *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /external", panel.Status)
	install(mux, preview{})
	return mux
}

func install(mux *http.ServeMux, panel preview) {
	mux.HandleFunc("GET /preview", panel.Status)
}
