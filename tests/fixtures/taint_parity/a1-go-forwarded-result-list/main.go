package main

import "os/exec"

func pair() (string, string) {
    t := acquire() // @SRC
    return "fixed", t
}

func forward() (string, string) {
    return pair()
}

func main() {
    _, out := forward()
    exec.Command(out) // @SINK
}

func acquire() string { return "value" }
