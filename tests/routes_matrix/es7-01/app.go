package fixture

import "net/http"

func list(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusOK) }
func save(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusCreated) }
func ping(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusNoContent) }

func NewMux() *http.ServeMux {
	child := http.NewServeMux()
	child.HandleFunc("GET /items", list)
	child.HandleFunc("POST /items", save)
	child.HandleFunc("/ping", ping)

	root := http.NewServeMux()
	root.Handle("GET /browse/", http.StripPrefix("/browse", child))
	root.Handle("/manage/", http.StripPrefix("/manage", child))
	return root
}
