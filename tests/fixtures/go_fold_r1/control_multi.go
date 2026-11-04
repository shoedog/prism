package p
func f(a int) int { // parameter
b := 2 // outer
{
a, b := b, a // multi
println(a, b) // inner
}
return a + b // result
}
