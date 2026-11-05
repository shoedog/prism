package fixture

type A struct { b, c string }
type S struct { f, g string; a A }

func scenario(s []string, i int, x string) {
	s[i] = x
	useSlice(s)
}
