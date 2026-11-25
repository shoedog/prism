package main
func caseFn() {
    t := src()
    d := map[string]S{"u": {n: "s0"}, "w": {n: "s0"}}
    d["u"] = S{n: t} // W
    sink(d["w"].n) // R
}

type S struct {
    n string
}
