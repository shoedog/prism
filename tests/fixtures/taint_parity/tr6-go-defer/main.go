package main
import "os/exec"
func run(x string) {
    exec.Command(x) // @SINK
}
func main() {
    t := acquire() // @SRC
    defer run(t)
    t = "fixed"
}
func acquire() string { return "value" }
