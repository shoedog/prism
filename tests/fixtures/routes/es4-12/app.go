package routes

import "net/http"

func tunnelStatus(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func gatewayStatus(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func NewApp() *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("CONNECT /gateway//status", tunnelStatus)
	mux.HandleFunc("GET /gateway/status", gatewayStatus)
	return mux
}
