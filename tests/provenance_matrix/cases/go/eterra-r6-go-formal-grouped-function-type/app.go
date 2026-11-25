package main
func f(first, second string, apply func(string) string) string {
  return apply(first) + second
}
