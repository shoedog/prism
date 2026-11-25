package main
func caseFn() {
    t := src()
    d := map[string]S{"u": {n: "s0"}}
    d["u"] = S{n: t} // W
    x := d["u"].n // R
    sink(x)
}

type S struct {
    n string
}
