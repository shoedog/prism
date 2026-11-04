package p

func f(d []byte, a bool) int {
	i := len(d)
	if a {
		d[i] = 1
	} else {
		d[i] = 0
	}
	i--
	d[i] = 0x30
	d[i] = 0x31
	return i
}
