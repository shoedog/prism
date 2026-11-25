package main
import "os/exec"
type Runner interface { Run(string) string }
type Keep struct{}
type Drop struct{}
func (Keep) Run(x string) string { return x }
func (Drop) Run(x string) string { return "fixed" }
func use(r Runner) {
    t := acquire() // @SRC
    out := r.Run(
        t,
    )
    exec.Command(out) // @SINK
}
func main() { use(Keep{}); use(Drop{}) }
func acquire() string { return "value" }
