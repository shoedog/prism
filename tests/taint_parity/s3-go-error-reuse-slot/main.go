package main

import "os/exec"

func read(x string) (string, error) { return x, nil }
func check(x string) error { return nil }
func entry() {
    t := acquire() // @SRC
    value, err := read(t)
    if err != nil { return }
    if err = check(value); err != nil { return }
    exec.Command(value) // @SINK
}
func acquire() string { return "sample" }
func main() {}
