package fixture
import "net/http"
func run() {
  http := struct{ Get func(string) string }{Get: func(string) string { return "ok" }}
  value := http.Get("x")
  consume(value)
}
func consume(...any) {}
