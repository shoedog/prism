package routes

import "net/http"

type report struct{}

func (*report) render(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func open() (*report, error) { return &report{}, nil }

func App() (*http.ServeMux, error) {
	mux := http.NewServeMux()
	selected, err := open()
	if err != nil {
		return nil, err
	}
	mux.HandleFunc("GET /report", selected.render)
	return mux, nil
}
