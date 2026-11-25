package main
import "os/exec"
func main() {
    t := acquire() // @SRC
    run := func() {
        exec.Command(t) // @SINK
    }
    run()
}
func acquire() string { return "value" }
