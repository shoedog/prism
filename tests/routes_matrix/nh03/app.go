package fixture

import "net/http"

type healthHandler struct{}

func (h healthHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {}

func NewApp() *http.ServeMux {
	mux := http.NewServeMux()
	v := healthHandler{}
	mux.Handle("/x", v)
	return mux
}
