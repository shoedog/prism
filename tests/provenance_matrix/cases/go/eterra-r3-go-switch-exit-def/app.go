package fixture
import "os"
func run(kind int) string {
  value := os.Getenv("MODE")
  switch kind {
  case 1:
    value = "fixed"
    return value
  }
  return value // final
}
