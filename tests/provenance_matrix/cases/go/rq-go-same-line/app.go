package fixture

import "net/http"

func read(r *http.Request) (string, int) {
    name := r.FormValue("q"); count := 7
    return name, count
}
