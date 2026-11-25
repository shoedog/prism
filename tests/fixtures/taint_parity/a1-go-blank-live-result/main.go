package main

import "os/exec"

func pair() (string, string) {
    t := acquire() // @SRC
    return "fixed", t
}

func main() {
    _, out := pair()
    exec.Command(out) // @SINK
}

func acquire() string { return "value" }
