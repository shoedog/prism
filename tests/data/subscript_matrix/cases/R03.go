package main
func caseFn() {
    t := src()
    _ = t
    d := map[string]string{"a": "s0", "b": "s0"}
    d["b"] = "s" // W
    f(d["b"]) // R
}

func f(p string) {
    sink(p)
}
