package routes

import (
	"net/http"

	"example.org/routes/endpoints"
)

type panel struct{ endpoints.Panel }

func NewMux(p panel) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /explicit", p.Panel.Status)
	mux.HandleFunc("GET /promoted", p.Status)
	return mux
}
