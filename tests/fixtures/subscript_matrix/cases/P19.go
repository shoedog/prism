package main
func caseFn() {
    t := src()
    d := map[string]string{"a": t, "b": "safe"} // W
    sink(d["b"]) // R
}
