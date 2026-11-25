package routes

import "net/http"

type controller struct{}

func (*controller) status(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func App() *http.ServeMux {
	mux := http.NewServeMux()
	services := struct{ dashboard *controller }{dashboard: &controller{}}
	mux.HandleFunc("GET /dashboard", services.dashboard.status)
	return mux
}
