package main
func caseFn() {
    t := src()
    s := S{x: "s0"}
    s.x = t
    m := map[string]S{"k": {x: "s0"}}
    m["k"] = s // W
    sink(m["k"].x) // R
}

type S struct {
    x string
}
