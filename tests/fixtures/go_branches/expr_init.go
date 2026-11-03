package p
func f(x int, n int, a bool, b bool, v interface{}, ch chan int) int {
switch t := n; t {
case 1:
x = 1 // def
default:
x = 2 // sibling kill
}
return x // use
}
