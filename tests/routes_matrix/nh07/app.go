package fixture

import "net/http"

func h(w http.ResponseWriter, r *http.Request) {}

func NewApp(versionPath string) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc(versionPath, h)
	return mux
}
