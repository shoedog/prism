package routes

import "net/http"

type worker struct{}

func (*worker) Status(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNoContent)
}

type settings struct {
	worker   *worker
	disabled bool
}

func NewMux(s settings) *http.ServeMux {
	mux := http.NewServeMux()
	worker, disabled := s.worker, s.disabled
	if !disabled {
		mux.HandleFunc("GET /status", worker.Status)
	}
	return mux
}
