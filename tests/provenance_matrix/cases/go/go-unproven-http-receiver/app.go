package fixture

import "net/http"

var _ = http.MethodGet

type localClient struct{}
func (localClient) Get(string) string { return "fixed" }

func (http localClient) read() string {
    value := http.Get("https://example.invalid")
    return value
}
