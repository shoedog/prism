package fixture
import "os"
func run() {
  value := os.Getenv("MODE")
  println(value, value)
}
