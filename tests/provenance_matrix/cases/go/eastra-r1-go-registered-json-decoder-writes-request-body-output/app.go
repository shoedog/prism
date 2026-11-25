package fixture
import (
    "encoding/json"
    "net/http"
)
func routes() *http.ServeMux {
    mux := http.NewServeMux()
    mux.HandleFunc("POST /edition", func(w http.ResponseWriter, req *http.Request) {
        var label string
        decodeErr := json.NewDecoder(req.Body).Decode(&label)
        if decodeErr != nil { return }
        _ = label
    })
    return mux
}
