package main

import "os/exec"

func pair() (string, string) {
    t := acquire() // @SRC
    return "fixed", t
}

func main() {
    var holder struct{ first string }
    var out string
    holder.first, out = pair()
    exec.Command(out) // @SINK
}

func acquire() string { return "value" }
