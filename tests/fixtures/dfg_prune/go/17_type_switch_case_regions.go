package fixture
type S struct { f int }
var x int
func f(v interface{}) {
switch v.(type) {
case int:
x = 1 // a
use(x) // ua
case string:
x = 2 // b
use(x) // ub
default:
x = 3 // c
use(x) // uc
}
}
