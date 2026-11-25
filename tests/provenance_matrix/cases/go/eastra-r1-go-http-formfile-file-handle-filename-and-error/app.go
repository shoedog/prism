package fixture
import "net/http"
func register(mux *http.ServeMux) {
    mux.HandleFunc("POST /upload", func(w http.ResponseWriter, incoming *http.Request) {
        stream, header, err := incoming.FormFile("cover")
        if err != nil { return }
        defer stream.Close()
        name := header.Filename
        _, _ = stream, name
    })
}
