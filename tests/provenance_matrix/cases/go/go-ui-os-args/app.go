package fixture

import "os"

func read() string {
    term := os.Args[1]
    return term
}
