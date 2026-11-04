package fixture
type S struct { f string }
func scenario(p S, x string, m map[S]string) {
 p.f = x
 m[p] = "value"
}
