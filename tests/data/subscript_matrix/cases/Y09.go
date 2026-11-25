package main
func caseFn() {
    t := src()
    xs := []string{"s0", "s0"}
    xs[0] = t // W
    sink(xs) // R
}
