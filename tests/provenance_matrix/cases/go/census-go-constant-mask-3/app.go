package fixture

import "os"

func read() string {
    var fixed, value = 0, os.Getenv("MODE")
    _ = fixed
    return value
}
