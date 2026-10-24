package main

func f(value any) {
	x := source()
	switch x := x.(type) {
	case int:
		sink(x)
	}
	sink(x)
}
