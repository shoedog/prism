package p
type S struct { a int }
func f(a bool, p *S) int {
q := p
q.a = 1 // def
switch a {
case true:
return 0
default:
return 0
}
return p.a // use
}
