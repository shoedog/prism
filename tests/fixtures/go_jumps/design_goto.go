package main

func g(a int) int {
	x := a
	y := 0
again:
	x = x + 1
	y = x
	if y < 10 {
		goto again
	}
	x = 0
	return x + y
}
