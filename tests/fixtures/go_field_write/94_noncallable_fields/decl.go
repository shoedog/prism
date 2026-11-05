package pkg

type Inner struct{}

type S struct {
	n   int
	s   []int
	m   map[string]int
	c   chan int
	q   *Inner
	i   interface{}
	e   error
	st  struct{}
	arr [2]int
}
