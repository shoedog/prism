package main

import "os/exec"

func source() (x string) {
    t := acquire() // @SRC
    x = t
    return x
}
func relay() (x string) {
    x = source()
    return x
}
func wrap() (x string) {
    x = relay()
    return x
}
func finish(x string) {
    exec.Command(x) // @SINK
}
func main() {
    x := wrap()
    finish(x)
}
func acquire() string { return "sample" }
