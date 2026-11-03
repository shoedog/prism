package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
switch i {
case 1:
fallthrough
case 2:
g() // probe
{
use(i) // target
}
}
return x // use
}
