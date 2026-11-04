package fixture

type A struct { b, c string }
type S struct { f, g string; a A }

func scenario(p *S, x string, c bool) {
	p.f = x
	if c {
	p = &S{}
	} else {
	p = &S{}
	}
	use(p)
}
