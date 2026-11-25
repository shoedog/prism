package main
import "os/exec"
func run(args ...string) {
    exec.Command(args[0]) // @SINK
}
func main() {
    t := acquire() // @SRC
    run(t)
}
func acquire() string { return "value" }
