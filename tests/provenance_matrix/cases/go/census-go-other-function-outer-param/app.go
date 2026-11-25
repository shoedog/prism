package fixture

import "net/http"

func source(r *http.Request) string {
    value := r.FormValue("q")
    return value
}

func plain(value string) string {

    return value
}
