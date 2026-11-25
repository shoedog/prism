package sample
import "net/http"
func emit(w http.ResponseWriter, r *http.Request, input string) {
    t /* @SRC */ := input; http.ServeFile(w, r, t) // @SINK
    observe(t)
}
func observe(value string) { _ = value }
