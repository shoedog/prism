package main
func caseFn() {
    t := src()
    m := map[string]string{"k": "s0"}
    m["k"] = t // W
    sink(m["k"]) // R
}
