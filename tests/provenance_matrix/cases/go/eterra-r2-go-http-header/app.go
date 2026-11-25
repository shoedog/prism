package main
import "net/http"
func handler(w http.ResponseWriter, r *http.Request) {
 token := r.Header.Get("Authorization")
 println(token)
}
