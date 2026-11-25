package main
func caseFn() {
    t := src()
    d := map[string]S{"u": {n: "s0"}}
    d["u"] = S{n: t} // W
    sink(d["u"].get("n")) // R
}

type S struct {
    n string
}

func (s S) get(key string) string {
    if key == "n" {
        return s.n
    }
    return "s0"
}
