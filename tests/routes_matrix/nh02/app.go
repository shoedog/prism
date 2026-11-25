package fixture

import "net/http"

var App = http.DefaultServeMux

func init() {
	http.HandleFunc("/w", func(w http.ResponseWriter, r *http.Request) {})
}
