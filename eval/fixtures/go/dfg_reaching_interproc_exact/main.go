package main
func callee(p int) { sink(p) }
func caller() {
	x := source()
	callee(x)
}
