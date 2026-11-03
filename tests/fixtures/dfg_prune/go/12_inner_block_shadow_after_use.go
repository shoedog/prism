package fixture
type S struct { f int }
var x int
func f() {
x := 1 // a
{
y := x // early
x := 2 // b
use(x, y) // inner
}
use(x) // u
}
