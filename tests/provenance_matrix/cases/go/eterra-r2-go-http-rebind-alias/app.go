package main
import h "net/http"
func run() {
 h := struct{ Get func(string) string }{Get: func(string) string { return "fixed" }}
 value := h.Get("https://example.org")
 println(value)
}
