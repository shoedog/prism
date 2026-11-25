package main
import "os/exec"
type Runner struct { cmd string }
func (r *Runner) Run() {
    exec.Command(r.cmd) // @SINK
}
func main() {
    t := acquire() // @SRC
    r := &Runner{}
    send := r.Run
    r.cmd = t
    send()
}
func acquire() string { return "value" }
