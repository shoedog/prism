package fixture
import "os"
func capture() string {
    mode := os.Getenv("MODE")
    defer func() { mode = "fixed" }()
    return mode
}
