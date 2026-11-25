package fixture

import "net/http"

func current(w http.ResponseWriter, request *http.Request) *http.Request {
    return request
}
