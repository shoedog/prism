package main
import "os/exec"
type Runner interface { Run(string) }
type A struct{}
type B struct{}
func (A) Run(x string) {
    exec.Command(x) // @SINK
}
func (B) Run(x string) { _ = x }
func choose(flag bool) Runner {
    if flag { return A{} }
    return B{}
}
func entry(r Runner) {
    t := acquire() // @SRC
    r.Run(
        t); A{}.Run("fixed")
}
func main() { entry(choose(true)) }
func acquire() string { return "value" }
