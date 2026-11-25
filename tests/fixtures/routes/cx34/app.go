package fixture

import "net/http"

type healthCheck struct{}

func (healthCheck) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

func NewApp(pattern string) *http.ServeMux {
	mux := http.NewServeMux()
	mux.Handle(pattern, healthCheck{})
	return mux
}
