package p
type S struct { a int }
func f(a bool, p *S) int {
q := p
q.a = 1 // def
if a {
return 0
} else {
return 0
}
return p.a // use
}
