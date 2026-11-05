package fixture

type A struct { b, c string }
type S struct { f, g string; a A }

func scenario(m map[string]string, k, x string) {
	m[k] = x
	useMap(m)
}
