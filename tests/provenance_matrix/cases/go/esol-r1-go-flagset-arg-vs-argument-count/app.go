package fixture

import (
    "flag"
    "os"
)

func load() (string, int) {
    flags := flag.NewFlagSet("preview", flag.ContinueOnError)
    if err := flags.Parse(os.Args[1:]); err != nil { return "", 0 }
    word := flags.Arg(0)
    count := flags.NArg()
    return word, count
}
