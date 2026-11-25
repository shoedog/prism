package main
import "os/exec"
type Runner struct{}
func (Runner) Run(x string) {
    exec.Command(x) // @SINK
}
func main() {
    t := acquire() // @SRC
    r := Runner{}
    next := r.Run
    next(t)
}
func acquire() string { return "value" }
