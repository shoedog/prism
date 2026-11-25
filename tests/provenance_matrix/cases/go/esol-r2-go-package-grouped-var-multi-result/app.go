package fixture
import "os"
var (
    mode, exists = os.LookupEnv("MODE")
    title = "Catalog"
)
func snapshot() (string, bool, string) {
    return mode, exists, title
}
