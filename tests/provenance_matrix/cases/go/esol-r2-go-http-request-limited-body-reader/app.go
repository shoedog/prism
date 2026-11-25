package fixture
import (
    "io"
    "net/http"
)
func register() {
    http.HandleFunc("/submit", func(w http.ResponseWriter, incoming *http.Request) {
        bounded := http.MaxBytesReader(w, incoming.Body, 1024)
        body, err := io.ReadAll(bounded)
        _, _ = body, err
    })
}
