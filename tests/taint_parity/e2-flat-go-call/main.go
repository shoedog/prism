package sample
import "fmt"
func f(inp string) {
  t := inp // @SRC
  fmt.Sprintf(t) // @SINK
}
