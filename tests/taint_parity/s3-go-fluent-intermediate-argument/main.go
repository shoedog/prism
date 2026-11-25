package main

import "os/exec"

type query struct{ x string }
func start() query { return query{} }
func (q query) With(x string) query { q.x = x; return q }
func (q query) Tail() string { return q.x }
func entry() {
    t := acquire() // @SRC
    x := start().
        With(t).
        Tail()
    exec.Command(x) // @SINK
}
func acquire() string { return "sample" }
func main() {}
