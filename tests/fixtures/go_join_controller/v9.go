package p

func f(d []byte, a int) int {
	i := len(d)
	switch a {
	case 1:
		d[i] = 1
	default:
		d[i] = 0
	}
	i--
	d[i] = 0x30
	return i
}
