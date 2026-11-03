package fixture
type S struct { f int }
var x int
func f(x int, n int) {
switch n {
case 1:
x = 1 // a
use(x) // ua
case 2:
x = 2 // b
use(x) // ub
}
}
