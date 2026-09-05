package main
type I interface { callee(int) }
func caller(i I) {
	x := source()
	i.callee(x)
}
