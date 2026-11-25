package fixture
import "os"
var snapshot = edition
var edition = os.Getenv("EDITION")
func label() string { return snapshot }
