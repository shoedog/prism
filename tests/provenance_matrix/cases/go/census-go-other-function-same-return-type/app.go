package fixture

import "os"

func source() string {
    value := os.Getenv("MODE")
    return value
}

func plain() string {
    value := "ready"
    return value
}
