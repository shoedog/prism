package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
switch i {
case 1:
fallthrough
case 2:
g() // arm
}
z := 1
return x // use
}
