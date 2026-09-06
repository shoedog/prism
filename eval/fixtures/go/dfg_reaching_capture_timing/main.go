package main
func f() {
	x := source()
	go func() { sink(x) }()
	x = clean()
}
