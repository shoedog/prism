package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
if a {
switch i {
case 1:
g() // probe
;
default:
g() // arm
}
} else {
g() // arm
}
return x // use
}
