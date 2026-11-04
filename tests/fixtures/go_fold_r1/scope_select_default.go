package p
func f(n int, value interface{}, ch, other chan int) int {
v := 0 // outer
for n > 0 {
select {
case <-ch:
v := 10 // shadow
println(v) // inner
default:
v = v + 1 // write
}
}
return v // result
}
