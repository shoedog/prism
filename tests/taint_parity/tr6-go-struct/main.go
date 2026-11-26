package main
import "os/exec"
type Payload struct { Cmd string; Safe string }
func run(p Payload) {
    exec.Command(p.Cmd) // @SINK
}
func main() {
    t := acquire() // @SRC
    run(Payload{Cmd: t, Safe: "fixed"})
}
func acquire() string { return "value" }
