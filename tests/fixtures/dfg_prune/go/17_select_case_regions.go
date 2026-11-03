package fixture
type S struct { f int }
var x int
func f(ch chan int, n int) {
select {
case <-ch:
x = 1 // a
use(x) // ua
case ch <- n:
x = 2 // b
use(x) // ub
default:
x = 3 // c
use(x) // uc
}
}
