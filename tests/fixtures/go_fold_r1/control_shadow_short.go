package p
func f(x int) int { // parameter
{
x := 1 // shadow
println(x) // inner
}
return x // result
}
