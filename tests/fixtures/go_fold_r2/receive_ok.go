package p
func f(ch chan int, n int) bool {
v := false // outer
for n > 0 {
select {
case _, v := <-ch: // binding
println(v) // inner
default:
v = !v // write
}
}
return v
}
