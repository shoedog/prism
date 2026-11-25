package main

import "os/exec"

type Runner interface { Run(...string) }
type A struct{}
type B struct{}
func (A) Run(xs ...string) {
    x := xs[0]
    exec.Command(x) // @SINK
}
func (B) Run(xs ...string) { observe(xs) }
func entry(r Runner) {
    t := acquire() // @SRC
    xs := []string{t}
    r.Run(
        xs...,
    )
}
func main() { entry(A{}); entry(B{}) }
func acquire() string { return "sample" }
func observe(any) {}
