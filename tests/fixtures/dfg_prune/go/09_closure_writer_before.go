package fixture
type S struct { f int }
var x int
func f() {
g := func() {
x = 9 // a
}
x = 2 // b
g()
use(x) // u
}
