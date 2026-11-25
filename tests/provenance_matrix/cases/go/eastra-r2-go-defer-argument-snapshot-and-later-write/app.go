package fixture
import "os"
func logEdition(write func(string)) {
    if len(os.Args) < 2 { return }
    edition := os.Getenv("EDITION")
    defer write(edition)
    edition = os.Args[1]
    write(edition)
}
