package routes

import "net/http"

type Nested struct {
	worker Deep
}

func (Nested) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusResetContent)
}

type layer struct{ Deep }

type composite struct {
	Near
	layer
	nested Nested
}

func NewMux(c composite) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /nearest", c.Status)
	mux.HandleFunc("GET /worker", c.nested.worker.Status)
	return mux
}
