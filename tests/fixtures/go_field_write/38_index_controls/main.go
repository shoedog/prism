package fixture
func scenario(m map[string]string, s []string, k string, i int, x string) {
 m[k] = x
 s[i] = x
 useString(m[k])
 useString(s[i])
 useMap(m)
 useSlice(s)
}
