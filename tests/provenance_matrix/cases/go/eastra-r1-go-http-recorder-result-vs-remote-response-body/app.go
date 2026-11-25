package fixture
import (
    "io"
    "net/http"
    "net/http/httptest"
)
func compare() ([]byte, []byte, error) {
    local := httptest.NewRecorder()
    local.WriteString("sample")
    preview := local.Result()
    defer preview.Body.Close()
    response, err := http.Get("https://example.invalid/edition")
    if err != nil { return nil, nil, err }
    defer response.Body.Close()
    actual, err := io.ReadAll(response.Body)
    if err != nil { return nil, nil, err }
    fixed, err := io.ReadAll(preview.Body)
    if err != nil { return nil, nil, err }
    return actual, fixed, nil
}
