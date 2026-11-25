package fixture

import "os"

func load() (string, bool) {
    var mode = os.Args[0]
    mode, exists := os.LookupEnv("MODE")
    return mode, exists
}
