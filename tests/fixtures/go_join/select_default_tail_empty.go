package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
select {
case <-ch:
g() // probe
;
default:
g() // arm
}
return x // use
}
