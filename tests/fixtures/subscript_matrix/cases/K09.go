package main
func caseFn() {
    t := src()
    m := map[string]string{"k": "s0"}
    m["\x6b"] = t // W
    sink(m["k"]) // R
}
