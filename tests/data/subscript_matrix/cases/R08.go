package main
func caseFn() {
    t := src()
    d := map[string]string{"a": "s0"}
    c := true
    if c {
        d["a"] = t // W
    } else {
        d["a"] = "s"
    }
    sink(d) // R
}
