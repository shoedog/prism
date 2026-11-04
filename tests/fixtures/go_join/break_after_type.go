package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
switch i {
case 1:
break // arm
default:
g() // arm
}
type T int
return x // use
}
