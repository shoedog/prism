package p
func f(a, b bool, i int, v interface{}, ch chan int, xs []int) int {
x := seed() // def
select {
case <-ch:
g() // arm
default:
g() // arm
}
i++
// intervening comment
;
type T int
i--
return x // use
}
