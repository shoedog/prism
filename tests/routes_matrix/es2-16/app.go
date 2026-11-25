package routes

import "net/http"

type endpoint interface {
	Status(http.ResponseWriter, *http.Request)
}

type controller struct{}

func (*controller) Status(w http.ResponseWriter, r *http.Request) { w.WriteHeader(204) }

func NewRouter(input endpoint) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /interface", input.Status)
	switch selected := input.(type) {
	case *controller:
		mux.HandleFunc("GET /concrete", selected.Status)
	}
	return mux
}
