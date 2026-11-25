package fixture
import "net/http"
func routes() *http.ServeMux {
    mux := http.NewServeMux()
    mux.HandleFunc("GET /catalog/{label}", func(w http.ResponseWriter, incoming *http.Request) {
        label := incoming.PathValue("label")
        _, _ = w.Write([]byte(label))
    })
    return mux
}
