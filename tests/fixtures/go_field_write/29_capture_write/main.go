package fixture

type A struct { b, c string }
type S struct { f, g string; a A }

func scenario(p *S, x string) {
	f := func() {
	p.f = x
	}
	f()
	use(p)
}
