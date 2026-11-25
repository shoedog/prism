package fixture

import "os"

func read() string {

    value := os.Getenv("request-id")
    return value
}
