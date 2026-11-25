package fixture
import ("io"; "net/http")
func run() {
  resp, _ := http.Get("https://example.org")
  body, _ := io.ReadAll(resp.Body)
  consume(body)
}
func consume(...any) {}
