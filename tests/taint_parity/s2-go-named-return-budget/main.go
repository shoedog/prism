package main
import "os/exec"
func source() (x string) {
    x = acquire() // @SRC
    return
}
func wrap() string {
    v := source()
    return v
}
func main() {
    out := wrap()
    exec.Command(out) // @SINK
}
func acquire() string { return "value" }
