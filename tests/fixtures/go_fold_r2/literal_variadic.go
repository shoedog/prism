package p
func f(ch chan int, n int, b bool, y interface{}, xs []int) int {
v := 0 // outer
for n > 0 {
if b {
g := func(v ...int) { // binding
println(v) // inner
}
println(g)
println(v) // after_inner
} else {
v = v + 1 // write
}
}
return v // result
}
