package p
func f(x int, value interface{}) int { // parameter
switch x := 1; value.(type) {
case int:
_ = x
x = 4 // write
}
return x // result
}
