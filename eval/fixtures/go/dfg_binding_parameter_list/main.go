package main

func f() {
	x := source()
	_ = func(x int) {
		sink(x)
	}
	sink(x)
}
