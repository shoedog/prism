package pkg

type Inner struct{}
type A struct{ Run Inner }
type B struct{}

func (b *B) Run() {}

func use(p *B, x string) {
	p.f = x
	p.Run()
}
