package p
func f(ch chan int, n int, b bool, y interface{}, xs []int) int {
z := 0 // outer
for n > 0 {
switch v := y.(type) { // binding
case int:
v = 1 // write
case string:
println(v) // inner
}
z = z + 1
}
return z // result
}
