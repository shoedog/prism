package pkg

type F func()
type G F

type A struct{ run G }
