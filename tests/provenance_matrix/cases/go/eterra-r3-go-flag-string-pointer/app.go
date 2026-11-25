package fixture
import "flag"
func main() {
  name := flag.String("name", "guest", "name")
  flag.Parse()
  println(*name)
}
