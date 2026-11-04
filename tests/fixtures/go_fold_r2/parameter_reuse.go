package p
func f(v int, n int) int {
v, q := v + 1, 2 // binding
for n > 0 {
println(v) // inner
v = v + q // write
}
return v
}
