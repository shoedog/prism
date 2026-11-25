package fixture

import "net/http"

type handler struct {
	label string
}

func (h *handler) show(w http.ResponseWriter, r *http.Request) {}

func NewApp() *http.ServeMux {
	mux := http.NewServeMux()
	handler := &handler{label: "items"}
	mux.HandleFunc("GET /x", handler.show)
	return mux
}
