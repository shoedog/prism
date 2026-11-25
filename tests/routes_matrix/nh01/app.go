package fixture

import "net/http"

type handler struct {
	collection string
}

func (h *handler) create(w http.ResponseWriter, r *http.Request) {}

func NewApp() *http.ServeMux {
	mux := http.NewServeMux()
	h := &handler{collection: "users"}
	mux.HandleFunc("POST /v1/u", h.create)
	return mux
}
