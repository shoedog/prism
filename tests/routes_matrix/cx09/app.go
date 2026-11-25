package fixture

import "net/http"

func report(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func NewApp() *http.ServeMux {
	mux := http.NewServeMux()
	mux.Handle("POST /report", http.HandlerFunc(report))
	return mux
}
