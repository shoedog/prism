package main
func caseFn() {
    t := src()
    k := "b"
    d := map[string]string{"a": "s0", "b": "s0"}
    d[k] = t // W
    sink(d["a"]) // R
}
