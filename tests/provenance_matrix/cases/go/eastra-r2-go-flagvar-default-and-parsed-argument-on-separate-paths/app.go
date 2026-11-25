package fixture
import (
    "flag"
    "os"
)
func edition() string {
    var label string
    flags := flag.NewFlagSet("book", flag.ContinueOnError)
    flags.StringVar(&label, "edition", "preview", "edition name")
    if err := flags.Parse(os.Args[1:]); err != nil { return "" }
    if flags.NFlag() == 0 {
        return label
    }
    return label
}
