package main

func f(items []int) {
	x := source()
	for _, x = range items {
		observe(x)
	}
	sink(x)
}
