package main
func caseFn() {
    t := src()
    sink(h(t))
}

type S struct {
    n string
}

func h(t string) string {
    d := map[string]S{"u": {n: "s0"}}
    d["u"] = S{n: t} // W
    return d["u"].n // R
}
