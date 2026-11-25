package main

import "os/exec"

func dispatch(f func(string), x string) { f(x) }
func entry() {
    t := acquire() // @SRC
    value := "fixed"
    callback := func(received string) { value = received }
    dispatch(callback, t)
    exec.Command(value) // @SINK
}
func acquire() string { return "sample" }
func main() {}
