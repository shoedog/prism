package main

type Doer interface {
	Do()
}

type Concrete struct{}

func (c Concrete) Do() {}

type Holder struct {
	Doer
}

func run(h Holder) {
	h.Do()
}
