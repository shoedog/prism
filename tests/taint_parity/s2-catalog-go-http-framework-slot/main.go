package sample
import "net/http"
func emit(w http.ResponseWriter, r *http.Request, input string) {
    t := input // @SRC
    http.ServeFile(w, r, t) // @SINK
}
