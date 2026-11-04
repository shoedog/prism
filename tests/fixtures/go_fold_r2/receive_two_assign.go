package p
func f(ch chan int, n int, b bool, y interface{}, xs []int) int {
v := 0 // outer
for n > 0 {
var ok bool
select {
case v, ok = <-ch: // binding
println(v) // inner
println(ok)
default:
v = v + 1 // write
}
}
return v // result
}
