package main

type Inner struct{}

func (i *Inner) M() {}

type Outer struct {
	Field *Inner
}

func run(o *Outer) {
	o.Field.M()
}
