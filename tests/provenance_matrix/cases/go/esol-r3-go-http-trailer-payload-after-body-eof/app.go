package fixture

import (
  "io"
  "net/http"
)

func checksum() (string, error) {
  response, err := http.Get("https://example.invalid/catalog")
  if err != nil { return "", err }
  defer response.Body.Close()
  if _, err := io.Copy(io.Discard, response.Body); err != nil { return "", err }
  digest := response.Trailer.Get("Digest")
  return digest, nil
}
