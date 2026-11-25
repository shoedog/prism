package main
func caseFn() {
    t := src()
    xs := []string{t} // W
    sink(xs[0]) // R
}
