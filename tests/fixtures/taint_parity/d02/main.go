package main
import "os/exec"

type Inner struct{}
func (Inner) Run(c string) {
    cmd := exec.Command(c) // @SINK
    _ = cmd
}
type Outer struct { Inner }
func main() {
    t := input() // @SRC
    o := Outer{}
    o.Run(t)
}
func input() string { return "" }
