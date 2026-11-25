package main
func caseFn() {
    t := src()
    d := map[string]string{"a": "s0"}
    d["a"] = ""
    d["a"] += t // W
    sink(d) // R
}
