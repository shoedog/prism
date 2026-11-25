package routes

import "net/http"

type lookup interface {
	Find(http.ResponseWriter, *http.Request)
}

type panel struct{ lookup }
type ownPanel struct{ lookup }

func (ownPanel) Find(w http.ResponseWriter, r *http.Request) { w.WriteHeader(204) }

func NewRouter(promoted panel, direct ownPanel) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /direct", direct.Find)
	mux.HandleFunc("GET /promoted", promoted.Find)
	return mux
}
