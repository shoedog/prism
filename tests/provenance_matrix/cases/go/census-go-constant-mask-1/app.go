package fixture

import "net/http"

func read(r *http.Request) string {
    fixed := 1; value := r.FormValue("mode")
    _ = fixed
    return value
}
