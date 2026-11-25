package fixture

import "flag"

func read() *flag.FlagSet {
    result := flag.NewFlagSet("preview", flag.ContinueOnError)
    return result
}
