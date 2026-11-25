package routes

import "net/http"

type draft struct{ title string }
type printable struct{ title string }

func (*draft) show(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusAccepted)
}

func (*printable) show(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func App() *http.ServeMux {
	mux := http.NewServeMux()
	item := &draft{title: "invoice"}
	mux.HandleFunc("GET /draft", item.show)
	mux.HandleFunc("GET /print", (*printable)(item).show)
	return mux
}
