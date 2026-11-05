package fixture

type A struct { b, c string }
type S struct { f, g string; a A }

func scenario(p *S, x, y string) {
	p.f = x
	p.g = y
	use(p)
}
