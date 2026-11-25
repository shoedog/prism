package main
import "os/exec"
func run(x string, extras ...string) {
    exec.Command(x) // @SINK
}
func main() {
    t := acquire() // @SRC
    tail := []string{"fixed"}
    run(t, tail...)
}
func acquire() string { return "value" }
