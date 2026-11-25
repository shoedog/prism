package fixture
import "os"
func working() string {
    edition := os.Getenv("EDITION")
    return edition
}
