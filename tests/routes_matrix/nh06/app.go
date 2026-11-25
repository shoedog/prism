package fixture

import "net/http"

func asset(w http.ResponseWriter, r *http.Request) {}

func ready(w http.ResponseWriter, r *http.Request) {}

func NewApp() *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("/assets/{p...}", asset)
	mux.HandleFunc("/ready/{$}", ready)
	return mux
}
