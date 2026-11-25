package routes

import "net/http"

type controller struct{}

func (*controller) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

type setup struct{ controller *controller }

func NewMux(s setup) *http.ServeMux {
	mux := http.NewServeMux()
	direct := s.controller
	indirect := direct
	mux.HandleFunc("GET /direct", direct.Status)
	mux.HandleFunc("GET /indirect", indirect.Status)
	return mux
}
