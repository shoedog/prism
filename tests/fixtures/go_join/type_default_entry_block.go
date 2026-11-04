package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
switch v.(type) {
case int:
{
use(i) // target
}
g() // probe
default:
g() // arm
}
return x // use
}
