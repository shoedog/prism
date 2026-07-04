package main

type L4 struct{}

func (l *L4) M() {}

type Other struct{}

func (o Other) M() {}

type L3 struct {
	Next *L4
}

type L2 struct {
	Next *L3
}

type L1 struct {
	Next *L2
}

func run(o *L1) {
	o.Next.Next.Next.M()
}
