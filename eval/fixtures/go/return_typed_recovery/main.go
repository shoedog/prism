package main

type Demux struct{}

func (d *Demux) Init(n int) {}

func newDemux(a, b int) *Demux {
	return &Demux{}
}

func run() {
	d := newDemux(16, 16)
	d.Init(1)
}
