package fixture
import "net/http"
func fetch() (string, string, error) {
    req, err := http.NewRequest("GET", "https://example.invalid/edition", nil)
    if err != nil { return "", "", err }
    req.Header.Set("X-Edition", "preview")
    client := &http.Client{CheckRedirect: func(*http.Request, []*http.Request) error {
        return http.ErrUseLastResponse
    }}
    resp, err := client.Do(req)
    if err != nil { return "", "", err }
    defer resp.Body.Close()
    received := resp.Header.Get("X-Edition")
    sent := resp.Request.Header.Get("X-Edition")
    return received, sent, nil
}
