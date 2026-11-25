package fixture
import "net/http"
func routes() *http.ServeMux {
    mux := http.NewServeMux()
    mux.HandleFunc("GET /{edition}/{locale}", func(w http.ResponseWriter, req *http.Request) {
        req.SetPathValue("edition", "preview")
        edition := req.PathValue("edition")
        locale := req.PathValue("locale")
        _, _ = edition, locale
    })
    return mux
}
