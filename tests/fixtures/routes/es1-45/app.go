package routes

import "net/http"

type controller struct{ code int }

func (c *controller) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(c.code)
}

func NewMux() *http.ServeMux {
	mux := http.NewServeMux()
	c := &controller{code: http.StatusAccepted}
	mux.HandleFunc("GET /before", c.Status)
	c = &controller{code: http.StatusNoContent}
	mux.HandleFunc("GET /after", c.Status)
	return mux
}
