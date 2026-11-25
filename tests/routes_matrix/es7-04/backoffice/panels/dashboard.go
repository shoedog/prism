package panels

import "net/http"

type Dashboard struct{}

func (d *Dashboard) Summary(w http.ResponseWriter, _ *http.Request) {
	_, _ = w.Write([]byte("staff workload"))
}
