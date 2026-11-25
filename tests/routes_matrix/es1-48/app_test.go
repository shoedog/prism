package routes

import "net/http"

func (Controller) Diagnostic(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusAccepted)
}

func diagnosticMux(c Controller) *http.ServeMux {
	mux := NewMux(c)
	mux.HandleFunc("GET /diagnostic", c.Diagnostic)
	return mux
}
