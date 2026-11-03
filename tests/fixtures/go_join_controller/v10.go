package p

func f(d []byte, a bool, x int) int {
	i := len(d)
	if a {
		d[i] = 1
	} else {
		d[i] = 0
	}
	x++
	d[i] = 0x30
	return i + x
}
