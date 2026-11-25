package routes

import "net/http"

func items(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusOK) }
func check(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusNoContent) }

func NewMux() *http.ServeMux {
	child := http.NewServeMux( /* edge-relative routes */ )
	child.HandleFunc("GET /api/items", items)
	root := http.NewServeMux()
	root.Handle("/edge/api/", http.StripPrefix("/edge", child))
	root.HandleFunc("GET /edge/check", check)
	return root
}
