package fixture
import "os"
func run() {
  for _, arg := range os.Args {
    consume(arg)
  }
}
func consume(...any) {}
