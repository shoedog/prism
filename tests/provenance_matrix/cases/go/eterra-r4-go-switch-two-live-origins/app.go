package fixture
import ("os"; "net/http")
func run(r *http.Request, flag bool) {
  var value string
  switch flag {
  case true: value = r.FormValue("q")
  default: value = os.Getenv("MODE")
  }
  consume(value)
}
func consume(...any) {}
