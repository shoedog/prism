package example
import "fmt"
import "net/http"
import "github.com/gorilla/mux"
type Req = http.Request
func isolated() {
    t := acquire() // @SRC
    observe(t)
}
func read(gate *http.Request, other *Req) {
    value := mux.Vars(other)["key"]
    _ = fmt.Sprintf(value) // @SINK
}
