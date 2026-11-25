package fixture
import "net/http"
func run() {
  response, err := http.Head("https://example.test")
  status := http.StatusText(200)
  consume(response, err, status)
}
func consume(...any) {}
