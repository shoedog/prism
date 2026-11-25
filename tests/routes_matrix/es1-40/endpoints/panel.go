package endpoints

import "net/http"

type Panel struct{}

func (Panel) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}
