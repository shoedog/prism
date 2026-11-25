package main
func caseFn() {
    t := src()
    o := S{a: "s0"}
    o.a = t // W
    sink(o.a) // R
}

type S struct {
    a string
}
