package fixture

import "net/http"

func source(r *http.Request) string {
    value := r.FormValue("q")
    return value
}

func plain() string {
    value := "ready"
    return value
}
