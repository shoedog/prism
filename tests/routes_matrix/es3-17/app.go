package routes

import "net/http"

func stock(w http.ResponseWriter, _ *http.Request)  { w.WriteHeader(http.StatusOK) }
func legacy(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusAccepted) }

func App() *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /stock", stock)
	mux.HandleFunc("get /stock", legacy)
	return mux
}
