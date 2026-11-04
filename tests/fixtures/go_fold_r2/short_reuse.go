package p
func f(n int) int {
v := 0
for n > 0 {
println(v) // inner
v, q := v + 1, 2 // write
println(q)
}
return v
}
