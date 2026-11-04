package p
func f(ch chan int, n int, b bool, y interface{}, xs []int) int {
v := 0 // outer
for n > 0 {
if b {
if v := 1; b { // binding
println(v) // inner
}
println(v) // after_inner
} else {
v = v + 1 // write
}
}
return v // result
}
