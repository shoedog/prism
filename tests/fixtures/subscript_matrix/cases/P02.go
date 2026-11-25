package main
func caseFn() {
    t := src()
    d := map[string]string{"a": "s0", "b": "s0"}
    d["a"] = t // W
    d["b"] = "safe"
    sink(d["a"]) // R
}
