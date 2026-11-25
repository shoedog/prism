package fixture

import "flag"

func read() string {
    p := flag.String("mode", "", "mode")
    flag.Parse()
    return *p
}
