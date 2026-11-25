package fixture

import (
    "fmt"
    "net/http"
)

func routes() {
    http.HandleFunc("/catalog", func(reply http.ResponseWriter, incoming *http.Request) {
        account, passphrase, present := incoming.BasicAuth()
        _, _, _ = account, passphrase, present
        fmt.Fprint(reply, "ready")
    })
}
