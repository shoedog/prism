package main
func f() {
	v := outer()
	if v := source(); v != nil {
		sink(v)
	}
	sink(v)
}
