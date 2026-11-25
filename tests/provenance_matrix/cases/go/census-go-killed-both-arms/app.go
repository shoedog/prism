package fixture

import "net/http"

func read(r *http.Request, flag bool) string {
    value := r.FormValue("q")
    if flag {
        value = "ready"
    } else {
        value = "fixed"
    }
    return value
}
