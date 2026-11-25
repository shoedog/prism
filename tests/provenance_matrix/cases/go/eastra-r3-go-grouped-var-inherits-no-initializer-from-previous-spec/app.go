package catalog
import "os"
var (
    fromEnv = os.Getenv("EDITION")
    empty string
)
func labels() (string, string) {
    return fromEnv, empty
}
