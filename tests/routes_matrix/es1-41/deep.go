package routes

import "net/http"

type Deep struct{}

func (Deep) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}
