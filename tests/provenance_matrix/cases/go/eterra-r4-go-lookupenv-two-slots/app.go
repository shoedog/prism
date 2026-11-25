package fixture
import "os"
func run() {
  mode, present := os.LookupEnv("MODE")
  consume(mode, present)
}
func consume(...any) {}
