package main
import ("net/http"; "strings")
func run() {
 resp, _ := http.Post("https://example.org", "text/plain", strings.NewReader("hi"))
 println(resp)
}
