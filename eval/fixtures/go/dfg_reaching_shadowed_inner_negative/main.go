package main
func f() {
	x := source()
	{
		x := clean()
		sink(x)
	}
	x = clean()
	sink(x)
}
