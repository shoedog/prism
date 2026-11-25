package fixture

import "os"

func read() string {
    value := os.Getenv("MODE")
    _ = value
    value = "fixed"
    return value
}
