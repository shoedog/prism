package main
import "os"
func f(n int) {
  value := os.Getenv("MODE")
  for i := 0; i < n; i++ { value = "fixed" }
  _ = value
}
