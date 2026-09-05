package main
func f() {
	x := source()
	defer func() { sink(x) }()
	x = clean()
}
