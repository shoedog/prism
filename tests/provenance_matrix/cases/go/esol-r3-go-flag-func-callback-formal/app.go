package fixture

import (
  "flag"
  "fmt"
  "os"
)

func parse() error {
  options := flag.NewFlagSet("preview", flag.ContinueOnError)
  options.Func("label", "preview label", func(submitted string) error {
    fmt.Println(submitted)
    return nil
  })
  return options.Parse(os.Args[1:])
}
