package p
func f(x int) int { // parameter
{
var x = 1 // shadow
println(x) // inner
}
return x // result
}
