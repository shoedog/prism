package fixture

import "net/http"

func read(r *http.Request) string {
    q := r.URL.Query().Get("q")
    return q
}
