package routes

import "net/http"

type report struct{ status int }
type draft struct{ status int }

func (r *report) Show(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(r.status) }
func (d *draft) Show(w http.ResponseWriter, _ *http.Request)  { w.WriteHeader(d.status) }

func NewMux() *http.ServeMux {
	mux := http.NewServeMux()
	target := &report{status: http.StatusOK}
	mux.HandleFunc("GET /initial", target.Show)
	target, revisedStatus := &report{status: http.StatusAccepted}, http.StatusNoContent
	mux.HandleFunc("GET /revised", target.Show)
	{
		target, previewPath := &draft{status: revisedStatus}, "GET /preview"
		mux.HandleFunc(previewPath, target.Show)
	}
	return mux
}
