package routes

import "net/http"

type Near struct{}

func (Near) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusAccepted)
}
