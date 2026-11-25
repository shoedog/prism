package main

import "os/exec"

type inner struct{ Field string }
type request struct{ Options inner }
func run(p *request) {
    x := p.Options.Field
    exec.Command(x) // @SINK
}
func entry(){
    t := acquire() // @SRC
    run(
        &request{Options: inner{Field: t}},
    )
}
func acquire() string { return "sample" }
func main() {}
