package fixture
import web "net/http"
import "net/url"
func run() {
  response, err := web.PostForm("https://example.org", url.Values{"a": {"b"}})
  consume(response, err)
}
func consume(...any) {}
