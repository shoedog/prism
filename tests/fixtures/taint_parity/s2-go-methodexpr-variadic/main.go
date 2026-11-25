package main
import "os/exec"
type Runner struct{}
func (Runner) Run(head string, tail ...string) {
    exec.Command(tail[0]) // @SINK
}
func main() {
    t := acquire() // @SRC
    Runner.Run(Runner{}, "fixed", []string{t}...)
}
func acquire() string { return "value" }
