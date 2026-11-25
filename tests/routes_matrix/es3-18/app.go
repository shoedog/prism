package routes

import "net/http"

func index(w http.ResponseWriter, _ *http.Request)   { w.WriteHeader(http.StatusOK) }
func chapter(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusAccepted) }

func App() *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /manual/", chapter)
	mux.HandleFunc("GET /manual/{$}", index)
	return mux
}
