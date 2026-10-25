package main

func f(ch, dh chan int) {
	x := source()
	select {
	case <-ch:
		x := clean()
		sink(x)
	case <-dh:
		sink(x)
	}
	sink(x)
}
