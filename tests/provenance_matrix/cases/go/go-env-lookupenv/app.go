package fixture

import "os"

func read() (string, bool) {
    fixed := ""; mode, found := os.LookupEnv("MODE")
    _ = fixed
    return mode, found
}
