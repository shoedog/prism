package fixture

import "net/http"

func read(w http.ResponseWriter, r *http.Request) string {
    term := r.FormValue("config")
    return term
}
