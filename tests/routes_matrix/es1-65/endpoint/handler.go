package endpoint

import "net/http"

type Handler struct{}

func MakeHandler() *Handler { return &Handler{} }

func (*Handler) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}
