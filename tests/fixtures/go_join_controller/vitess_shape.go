package p

func f(d []byte, a bool, c bool) int {
	i := len(d)
	_ = i
	if c {
		i--
		d[i] = 2
	}
	if a {
		i--
		if a {
			d[i] = 1
		} else {
			d[i] = 0
		}
		i--
		d[i] = 0x30
	}
	return i
}
