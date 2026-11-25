package main
import "os/exec"

func g(c string) {
    cmd := exec.Command(c) // @SINK
    _ = cmd
}
func input() string { return "" }
func main() {
    t := input() // @SRC
    u := "fixed"
    g(t); g(u)
}
