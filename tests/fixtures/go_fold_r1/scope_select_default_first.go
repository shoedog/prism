package p
func f(n int, value interface{}, ch, other chan int) int {
v := 0 // outer
for n > 0 {
select {
default:
v := 10 // shadow
println(v) // inner
case <-ch:
v = v + 1 // write
}
}
return v // result
}
