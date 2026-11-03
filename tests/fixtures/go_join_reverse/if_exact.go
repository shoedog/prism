package p
type S struct { a int }
func f(a bool, p *S) int {
x := 1 // def
if a {
return 0
} else {
return 0
}
return x // use
}
