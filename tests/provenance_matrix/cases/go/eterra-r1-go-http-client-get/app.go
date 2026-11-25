package main
import (
 "fmt"
 "net/http"
)
func run() {
 client := &http.Client{}
 response, _ := client.Get("https://example.com")
 fmt.Println(response)
}
