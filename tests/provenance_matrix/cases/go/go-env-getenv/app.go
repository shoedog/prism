package fixture

import "os"

func read() string {
    mode := os.Getenv("MODE")
    return mode
}
