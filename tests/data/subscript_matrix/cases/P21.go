package main
func caseFn() {
    t := src()
    d := map[string]string{"a": "s0"}
    d["a"] = t // W
    d["a"] += "x"
    sink(d) // R
}
