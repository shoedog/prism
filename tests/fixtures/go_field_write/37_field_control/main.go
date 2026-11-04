package fixture
type S struct { f, g string }
func scenario(p *S, x string) {
 p.f = x
 useString(p.f)
 useString(p.g)
}
