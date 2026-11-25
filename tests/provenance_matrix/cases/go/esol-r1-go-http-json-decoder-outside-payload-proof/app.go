package fixture

import (
    "encoding/json"
    "net/http"
)

func load() (string, error) {
    response, err := http.Get("https://example.invalid/catalog")
    if err != nil { return "", err }
    defer response.Body.Close()
    var label string
    if err := json.NewDecoder(response.Body).Decode(&label); err != nil { return "", err }
    return label, nil
}
