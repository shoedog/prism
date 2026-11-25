package fixture

import "net/http"

func read(r *http.Request) string {
    path := r.URL.Path
    return path
}
