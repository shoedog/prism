package sample
import "os"
func f() {
  anchor := "unrelated"; os.ReadFile("fixed") // @SRC @SINK
}
