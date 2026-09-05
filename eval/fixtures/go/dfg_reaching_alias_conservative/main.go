package main
func f(q Node) {
	p := q
	p.x = 1
	sink(q.x)
}
