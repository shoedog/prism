package main
import (
 "fmt"
 "net/http"
)
func run() {
 response, err := http.Get("https://example.com")
 text := http.StatusText(200)
 fmt.Println(response, err, text)
}
