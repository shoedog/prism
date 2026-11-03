package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
switch i {
case 1:
break // arm
default:
i--
g() // probe
}
return x // use
}
