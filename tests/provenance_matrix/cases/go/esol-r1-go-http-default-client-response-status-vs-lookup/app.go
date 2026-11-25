package fixture

import "net/http"

func load() (string, string, error) {
    request, err := http.NewRequest(http.MethodGet, "https://example.invalid/catalog", nil)
    if err != nil { return "", "", err }
    response, err := http.DefaultClient.Do(request)
    if err != nil { return "", "", err }
    defer response.Body.Close()
    status := response.Status
    lookup := http.StatusText(response.StatusCode)
    return status, lookup, nil
}
