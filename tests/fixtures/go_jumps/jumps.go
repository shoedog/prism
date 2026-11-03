package fixture

func forward(x int, c bool) int {
 x = 1
 if c {
  goto L // jump:forward:forward_use
 }
 x = 2
L: // target:forward_use
 return x
}

func backward(x int) int {
L: // target:backward_use
 use(x)
 x = x + 1
 if x < 10 {
  goto L // jump:backward:backward_use
 }
 return x
}

func breakFor(x int) int {
 for x < 10 {
  x = 1
  break // jump:break_for:break_for_use
  x = 2
 }
 return x // target:break_for_use
}

func breakSwitch(x int) int {
 for x < 10 {
  switch x {
  default:
   x = 1
   break // jump:break_switch:break_switch_use
   x = 2
  }
  use(x) // target:break_switch_use
 }
 return x
}

func breakSelect(x int, ch chan int) int {
 select {
 case <-ch:
  x = 1
  break // jump:break_select:break_select_use
  x = 2
 }
 return x // target:break_select_use
}

func breakType(x int, v interface{}) int {
 switch v.(type) {
 default:
  x = 1
  break // jump:break_type:break_type_use
  x = 2
 }
 return x // target:break_type_use
}

func outerBreak(x int) int {
Outer:
 for x < 10 {
  for x < 5 {
   x = 1
   break Outer // jump:outer_break:outer_break_use
   x = 2
  }
  x = 3
 }
 return x // target:outer_break_use
}

func switchBreak(x int) int {
Outer:
 switch x {
 default:
  for x < 10 {
   x = 1
   break Outer // jump:switch_break:switch_break_use
   x = 2
  }
 }
 return x // target:switch_break_use
}

func selectBreak(x int, ch chan int) int {
Outer:
 select {
 case <-ch:
  for x < 10 {
   x = 1
   break Outer // jump:select_break:select_break_use
   x = 2
  }
 }
 return x // target:select_break_use
}

func continueSwitch(x int) int {
 for x < 10 { // target:continue_switch_header
  use(x)
  switch x {
  default:
   x = x + 1
   continue // jump:continue_switch:continue_switch_header
   x = 2
  }
 }
 return x
}

func outerContinue(x int) int {
Outer:
 for x < 10 { // target:outer_continue_header
  use(x)
  for x < 5 {
   x = x + 1
   continue Outer // jump:outer_continue:outer_continue_header
   x = 2
  }
 }
 return x
}

func enclosing(x int, c bool) int {
 x = 1
 if c {
  goto L // jump:enclosing:enclosing_use
 }
 x = 2
L: return x // target:enclosing_use
}

func noGoto(x int) int {
Unused:
 x = 1
 x = 2
 return x
}

func finalBreak() {
 for {
  break // noedge:final_break
 }
}

func finalBreakDefer() {
 defer cleanup() // target:deferred
 for {
  break // jump:final_break_defer:deferred
 }
}

func onlyJumps(x int) {
L: goto L // jump:only_jumps:only_jumps
}

func nested(x int) int {
L: x = 1 // target:outer_label
 f := func() {
 L: goto L // noedge:nested_local
 }
 if x > 0 {
  goto L // jump:outer_local:outer_label
 }
 f()
 return x
}
