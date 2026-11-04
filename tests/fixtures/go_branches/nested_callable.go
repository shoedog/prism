package p
func f(x int, n int, a bool, b bool, v interface{}, ch chan int) int {
if a {
g := func() {
switch n {
case 1:
x = 1
}
}
_ = g
} else if b {
x = 2 // def
}
return x // use
}
