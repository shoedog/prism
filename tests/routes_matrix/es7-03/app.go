package fixture

import "net/http"

type controller struct{}
type preview struct{}

func (c *controller) status(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func (p *preview) status(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusAccepted)
}

func (reports *controller) mount(mux *http.ServeMux, showPreview bool) {
	if reports := new(preview); showPreview {
		mux.HandleFunc("GET /preview", reports.status)
	}
	mux.HandleFunc("GET /reports", reports.status)
}

func NewMux(showPreview bool) *http.ServeMux {
	mux := http.NewServeMux()
	reports := &controller{}
	reports.mount(mux, showPreview)
	return mux
}
