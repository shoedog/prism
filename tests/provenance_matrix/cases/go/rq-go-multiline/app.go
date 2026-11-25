package fixture

import "net/http"

func read(r *http.Request) string {
    name :=
        r.FormValue(
            "q",
        )
    return name
}
