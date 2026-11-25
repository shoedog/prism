package fixture

import . "os"

func read() string {
    value := Getenv("MODE")
    return value
}
