package main
import "os/exec"
func split(x string) (string, string) { return "fixed", x }
func main() {
    t := acquire() // @SRC
    safe, out := split(t)
    _ = safe
    exec.Command(out) // @SINK
}
func acquire() string { return "value" }
