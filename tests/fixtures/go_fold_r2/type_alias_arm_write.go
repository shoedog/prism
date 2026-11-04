package p
func f(ch chan int, n int, b bool, y interface{}, xs []int) int {
v := 0 // outer
for n > 0 {
switch v := y.(type) { // binding
case int:
println(v) // inner
default:
v = 1 // write
}
}
return v // result
}
