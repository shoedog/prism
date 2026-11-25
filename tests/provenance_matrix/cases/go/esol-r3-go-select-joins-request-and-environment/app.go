package fixture

import (
  "net/http"
  "os"
)

func register(ready <-chan struct{}) {
  http.HandleFunc("/preview", func(out http.ResponseWriter, incoming *http.Request) {
    var label string
    select {
    case <-ready:
      label = incoming.FormValue("label")
    default:
      label = os.Getenv("PREVIEW_LABEL")
    }
    _, _ = out.Write([]byte(label))
  })
}
