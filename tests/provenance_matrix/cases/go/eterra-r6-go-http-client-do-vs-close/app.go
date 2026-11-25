package main
import "net/http"
func read() {
  client := &http.Client{}
  req, _ := http.NewRequest("GET", "https://example.com", nil)
  response, err := client.Do(req)
  closeErr := response.Body.Close()
  _, _, _ = response, err, closeErr
}
