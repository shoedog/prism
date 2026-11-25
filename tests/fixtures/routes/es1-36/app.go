package routes

import "net/http"

type controller struct{}

func (*controller) status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func NewMux(value any) *http.ServeMux {
	mux := http.NewServeMux()
	c := value.(*controller)
	mux.HandleFunc("GET /status", c.status)
	return mux
}
