package fixture

import "net/http"

func show(w http.ResponseWriter, r *http.Request) {}

func NewApp() *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("example.com/x", show)
	return mux
}
