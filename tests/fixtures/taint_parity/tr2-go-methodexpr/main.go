package main
import "os/exec"
type Runner struct{}
func (r Runner) Run(x string) {
    exec.Command(x) // @SINK
}
func main() {
    t := acquire() // @SRC
    Runner.Run(Runner{}, t)
}
func acquire() string { return "value" }
