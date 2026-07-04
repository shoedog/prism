package main

type Demux struct{}

func (d *Demux) Init(n int) {}

type Other struct{}

func (o Other) Init(n int) {}

func newDemux(a, b int) (*Demux, int, error) {
	return &Demux{}, 0, nil
}

func run() {
	d, _, _ := newDemux(16, 16)
	d.Init(1)
}
