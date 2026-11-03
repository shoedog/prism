package fixture

type T struct{}

func expressions(x []byte) {
	_ = new(string(x))
	_ = new(int(3))
	_ = new(T{})
	_ = new(T)
	_ = new(string)
	_ = make([]int, 3)
}

// Goyacc-shaped case-local definitions: U drops write-only pairs and D
// drops definitions outside the last dominating write's lexical window.
func parse(x []byte, n int) {
	_ = new(string(x))
	var yyLOCAL string // outer definition
	switch n {
	case 1:
		yyLOCAL = "old" // overwritten definition
		yyLOCAL = "current" // dominating definition
		use(yyLOCAL) // dominated read
	case 2:
		use(yyLOCAL) // outer read
	}
}
