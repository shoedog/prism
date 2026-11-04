package p
func f(ch chan int, n int, b bool, y interface{}, xs []int) int {
v := 0 // outer
for n > 0 {
select {
case v := <-ch: // binding
println(v) // inner
default:
v = v + 1 // write
}
}
return v // result
}
