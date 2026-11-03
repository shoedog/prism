package fixture

// The new goto route still traverses an invented cross-case sequential join.
func flagged(x, n int) int {
 switch n {
 case 1:
  x = 1
 case 2:
  goto L
 }
 x = 2
L: return x
}

func directContinue(x int) int {
 for x < 10 {
  use(x)
  x = x + 1
  continue
  x = 2
 }
 return x
}

func rangeBreak(x int, xs []int) int {
 for _, v := range xs {
  x = v
  break
  x = 2
 }
 return x
}

func labeledType(x int, v interface{}) int {
Outer:
 switch v.(type) {
 default:
  for x < 10 {
   x = 1
   break Outer
   x = 2
  }
 }
 return x
}
