package routes

import "net/http"

type controller struct{ path string }

func (controller) Show(w http.ResponseWriter, r *http.Request) { w.WriteHeader(204) }

func NewRouter(controllers []controller) *http.ServeMux {
	mux := http.NewServeMux()
	for _, item := range controllers {
		mux.HandleFunc(item.path, item.Show)
	}
	return mux
}
