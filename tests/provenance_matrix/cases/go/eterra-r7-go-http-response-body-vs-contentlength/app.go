package main
import ("io"; "net/http")
func load() {
 resp, _ := http.Get("https://example.test")
 body, _ := io.ReadAll(resp.Body)
 size := resp.ContentLength
 _, _ = body, size
}
