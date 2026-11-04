package p
func f(ch chan int, n int, b bool, y interface{}, xs []int) int {
v := 0 // outer
for n > 0 {
if b {
v, q := 1, 2 // binding
println(v) // inner
println(q)
} else {
v = v + 1 // write
}
}
return v // result
}
