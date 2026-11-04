package p
func f(n, limit int) int {
i := 0 // outer
for i < limit { // header
saved := i // saved
if n > 0 {
i = saved + 1 // write
} else {
i += 2 // second
}
}
return i // result
}
