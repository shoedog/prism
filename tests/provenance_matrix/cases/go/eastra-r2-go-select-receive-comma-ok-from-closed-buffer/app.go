package fixture
import "os"
func read() (string, bool) {
    values := make(chan string, 1)
    values <- os.Getenv("EDITION")
    close(values)
    select {
    case label, open := <-values:
        return label, open
    }
}
