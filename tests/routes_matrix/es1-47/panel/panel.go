package panel

import "net/http"

func Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusAccepted)
}
