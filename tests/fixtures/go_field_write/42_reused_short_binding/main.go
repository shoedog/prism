package fixture

type A struct { b, c string }
type S struct { f, g string; a A }

func scenario(p *S, x string) {
	p.f = x
	p, y := fresh(), 0
	useInt(y)
	use(p)
}
