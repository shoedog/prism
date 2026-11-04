package p
func f(x int, n int, a bool, b bool, v interface{}, ch chan int) int {
switch n {
case 1:
x = 1 // def
fallthrough
case 2:
default:
x = 2
}
return x // use
}
