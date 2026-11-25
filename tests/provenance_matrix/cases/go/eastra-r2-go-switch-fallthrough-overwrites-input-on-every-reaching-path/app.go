package fixture
import "os"
func edition(kind int) string {
    if len(os.Args) < 2 { return "" }
    var label string
    switch kind {
    case 1:
        label = os.Args[1]
        fallthrough
    case 2:
        label = os.Getenv("EDITION")
    default:
        return ""
    }
    return label
}
