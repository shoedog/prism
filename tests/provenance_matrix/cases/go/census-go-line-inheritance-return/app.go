package fixture

import "net/http"

func read(r *http.Request) (int, string) {
    count := len("fixture")
    return count, r.FormValue("mode")
}
