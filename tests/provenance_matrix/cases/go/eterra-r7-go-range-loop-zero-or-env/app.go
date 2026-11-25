package main
import "os"
func load(xs []int) {
 value := "fixed"
 for range xs { value = os.Getenv("MODE") }
 _ = value
}
