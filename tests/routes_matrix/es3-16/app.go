package routes

import "net/http"

type reporter interface {
	Status(http.ResponseWriter, *http.Request)
}

type draft struct{}

func (*draft) Status(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusAccepted)
}

type final struct{}

func (*final) Status(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func App(input reporter) *http.ServeMux {
	mux := http.NewServeMux()
	switch selected := input.(type) {
	case *draft, *final:
		mux.HandleFunc("GET /status", selected.Status)
	}
	return mux
}
