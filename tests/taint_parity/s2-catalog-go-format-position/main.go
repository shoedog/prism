package sample
import "fmt"
import "io"
func emit(w io.Writer, input string) {
    t := input // @SRC
    fmt.Fprintf(w, t, "value") // @SINK
}
