package main

import "os/exec"

func forget(v string) (out string) {
    if len(v) == 0 {
        return ""
    }
    return
}

func main() {
    t := acquire() // @SRC
    out := forget(t)
    exec.Command(out) // @SINK
}

func acquire() string { return "value" }
