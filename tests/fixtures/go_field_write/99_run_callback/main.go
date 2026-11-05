package fixture

type A struct { b, c string }
type S struct { f, g string; a A }

func scenario(p *S, q *S, x string, y string, r A, c bool) {
	each(func() { use(p) })
	p.f = x
	each(func() { use(p) })
}
