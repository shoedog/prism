package main

import "os/exec"

func identity(x string) string { return x }
func apply(x string) error {
    exec.Command(x) // @SINK
    return nil
}
func entry() {
    t := acquire() // @SRC
    if err := apply(identity(t)); err != nil { return }
}
func acquire() string { return "sample" }
func main() {}
