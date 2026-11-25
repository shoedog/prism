package fixture

import "os"

func read() string {
    value := "fixed"
    observe(value)
    value = os.Getenv("MODE")
    return value
}

func observe(string) {}
