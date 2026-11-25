package fixture

import "net/http"

type handler struct{}

func (h *handler) tokenHandler(w http.ResponseWriter, r *http.Request) {}

func withApiKeyAuth(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		next(w, r)
	}
}

func NewApp() *http.ServeMux {
	mux := http.NewServeMux()
	h := &handler{}
	mux.HandleFunc("/token", withApiKeyAuth(h.tokenHandler))
	return mux
}
