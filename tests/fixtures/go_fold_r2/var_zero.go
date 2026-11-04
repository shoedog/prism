package p
func f(ch chan int, n int, b bool, y interface{}, xs []int) int {
v := 0 // outer
for n > 0 {
if b {
var v int // binding
println(v) // inner
} else {
v = v + 1 // write
}
}
return v // result
}
