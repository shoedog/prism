package main
import "os/exec"
func run(xs ...string) {
    exec.Command(xs[0]) // @SINK
}
func main() {
    t := acquire() // @SRC
    tail := []string{t}
    run(tail...)
}
func acquire() string { return "value" }
