package fixture

import "net/http"

func list(w http.ResponseWriter, r *http.Request) {}

func NewApp(b string) *http.ServeMux {
	root := http.NewServeMux()
	a := http.NewServeMux( /* mounted routes */ )
	a.HandleFunc("/items", list)
	root.Handle(b+"/", http.StripPrefix(b, a))
	return root
}
