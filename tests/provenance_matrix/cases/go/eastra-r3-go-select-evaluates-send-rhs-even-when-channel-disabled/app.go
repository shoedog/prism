package catalog
import "os"
func edition() string {
    label := "preview"
    var disabled chan string
    select {
    case disabled <- func() string { label = os.Getenv("EDITION"); return "sent" }():
    default:
    }
    return label
}
