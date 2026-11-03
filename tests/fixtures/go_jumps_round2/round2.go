package p

func switchTail(x int, c bool) int {
 for c { // target:st-header
  use(x) // use:st
  switch x {
  default:
   x = 1 // def:st
   break // jump:st:st-header
  }
 }
 return x
}

func thenTail(x int, c bool) int {
 if c {
  for {
   x = 1 // def:then
   break // jump:then:then-use
  }
 } else {
  x = 2
 }
 return x // target:then-use use:then
}

func nestedTail(x int, a, b bool) int {
 for a {
  for b { // target:nested-header
   use(x) // use:nested
   switch x {
   case 1:
    x = 1 // def:nested
    break // jump:nested:nested-header
   }
  }
  x = 2
 }
 return x
}

// Synthetic ToPrinter shape: loop in then, conditional break, else overwrite.
func printer(value string, empty, match bool) string {
 if empty {
  for match {
   if match {
    value = "picked" // def:printer
    break // jump:printer:printer-use
   }
  }
 } else {
  value = "fallback"
 }
 use(value) // target:printer-use use:printer
 return value
}

func finalPlain(x int) {
 for {
  use(x) // use:final
  x = 1 // def:final
  break // noedge:final
 }
}

func finalDefer(x int) {
 defer use(x) // target:defer use:defer
 for {
  x = 1 // def:defer
  break // jump:defer:defer
 }
}

func missingCase(x int, c bool) int {
 for c {
  use(x) // use:case
  switch x {
  case 0:
   x = 2
   continue
  default:
   x = 1 // def:case
   continue
  }
 }
 return x
}

func acyclic(x int) int {
 goto Assign
Read:
 return x // use:acyclic
Assign:
 x = 1 // def:acyclic
 goto Read
}

func cyclic(x int, c bool) int {
Read:
 use(x) // use:cyclic
 x = 1 // def:cyclic
 if c {
  goto Read
 }
 return x
}

func elseIf(x int, a, b bool) int {
 for more() {
  x = 2
  if a {
   continue
  } else if b {
   continue
  } else {
   x = 1 // def:elseif
   continue
  }
 }
 return x // use:elseif
}

func elseRead(x int, c bool) int {
 if c {
  for {
   x = 1 // def:else-read
   break
  }
 } else {
  use(x) // use:else-read
 }
 return x
}
