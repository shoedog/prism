package p
func f(x int, n int, a bool, b bool, v interface{}, ch chan int) int {
x = 0
switch n {
case 1:
x = 1 // def
use(x)
case 2:
x = 2 // sibling kill
use(x)
}
return x // use
}
