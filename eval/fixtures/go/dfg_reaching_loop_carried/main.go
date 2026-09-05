package main
func f() {
	x := source()
	for ready() {
		sink(x)
		x = nextValue()
	}
}
