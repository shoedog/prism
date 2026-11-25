package fixture
import "os"
func run() {
  mode := os.Getenv("MODE")
  inner := func() { consume(mode) }
  inner()
}
func consume(...any) {}
