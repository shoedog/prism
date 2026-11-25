package fixture
import "net/http"
func routes() *http.ServeMux {
    mux := http.NewServeMux()
    mux.HandleFunc("/edition", func(w http.ResponseWriter, incoming *http.Request) {
        snapshot := incoming.Clone(incoming.Context())
        incoming.Header.Set("X-Edition", "preview")
        original := snapshot.Header.Get("X-Edition")
        current := incoming.Header.Get("X-Edition")
        println(original, current)
    })
    return mux
}
