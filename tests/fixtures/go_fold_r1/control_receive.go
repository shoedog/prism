package p
func f(ch chan int, n int) int {
v := 0 // outer
for n > 0 {
select {
case v := <-ch: // receive
println(v) // inner
default:
v = v + 1 // write
}
}
return v // result
}
