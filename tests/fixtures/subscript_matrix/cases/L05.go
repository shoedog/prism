package main
func caseFn() {
    t := src()
    k := "b"
    d := map[string]string{"a": "s0", "b": "s0"}
    d["a"] = t // W
    d[k] = "clean"
    sink(d["a"]) // R
}
