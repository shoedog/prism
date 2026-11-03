package main

type S struct{ union interface{} }

func parse(yyS []S, n int) S {
	var yyVAL S
	var yyDollar []S
again:
	switch n {
	case 1:
		yyDollar = yyS[0:1]
		var yyLOCAL int
		yyLOCAL = len(yyDollar)
		yyVAL.union = yyLOCAL
	case 2:
		yyDollar = yyS[1:2]
		var yyLOCAL string
		yyLOCAL = "two"
		yyVAL.union = yyLOCAL
	case 3:
		yyDollar = yyS[2:3]
		var yyLOCAL []int
		yyLOCAL = append(yyLOCAL, len(yyDollar))
		yyVAL.union = yyLOCAL
	}
	if n > 0 { goto again }
	return yyVAL
}
