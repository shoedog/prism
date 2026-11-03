package fixture
type S struct { f int }
var x int
func f() {
L: x = 1 // a
x = 2 // b
use(x) // u
goto L
}
