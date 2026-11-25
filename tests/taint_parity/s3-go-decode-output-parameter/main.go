package main

import "os/exec"

type input struct{ Value string }
type decoder struct{ value string }
func newDecoder(x string) decoder { return decoder{x} }
func (d decoder) Decode(dst *input) error { dst.Value = d.value; return nil }
func entry() {
    t := acquire() // @SRC
    var dst input
    if err := newDecoder(t).Decode(&dst); err != nil { return }
    x := dst.Value
    exec.Command(x) // @SINK
}
func acquire() string { return "sample" }
func main() {}
