package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
switch i {
case 1:
g() // arm
default:
g() // arm
}
{
use(i) // target
}
return x // use
}
