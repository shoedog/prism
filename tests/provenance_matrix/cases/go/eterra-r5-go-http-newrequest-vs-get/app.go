package fixture
import "net/http"
func run() {
  request, _ := http.NewRequest("GET", "https://example.org", nil)
  response, _ := http.Get("https://example.org")
  consume(request, response)
}
func consume(...any) {}
