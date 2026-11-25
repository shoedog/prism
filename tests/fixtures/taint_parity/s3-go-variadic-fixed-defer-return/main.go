package main

import "os/exec"

func echo(x string) string { return x }
func report(tag string, values ...string) {
    exec.Command(tag) // @SINK
}
func entry() {
    t := acquire() // @SRC
    defer report(echo(t), "fixed")
}
func main() { entry() }
func acquire() string { return "sample" }
