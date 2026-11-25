package main
func caseFn() {
    t := src()
    k := "a"
    d := map[string]string{"a": "s0", "b": "s0"}
    d["a"] = t // W
    sink(d[k]) // R
}
