package main
func caseFn() {
    t := src()
    const k = "a"
    m := map[string]string{"a": "s0"}
    m[k] = t // W
    sink(m["a"]) // R
}
