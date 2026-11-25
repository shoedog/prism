package fixture

import "net/http"

func read() string {
    result := http.StatusText(200)
    return result
}
