package main
func f() {
	x := source()
	defer sink(x)
	x = clean()
	return
}
