package fixture
type S struct { f int }
var x int
func f(c bool) {
x = 1 // a
if c { goto L }
x = 2 // b
L: use(x) // u
}
