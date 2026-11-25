package main
func caseFn() {
    t := src()
    d := map[string]S{"u": {n: "s0"}}
    d["u"] = S{n: t} // W
    f(d["u"].n) // R
}

type S struct {
    n string
}

func f(p string) {
    sink(p)
}
