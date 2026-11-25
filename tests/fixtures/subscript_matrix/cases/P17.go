package main
func caseFn() {
    t := src()
    d := map[string]string{"a": t} // W
    sink(d["a"]) // R
}
