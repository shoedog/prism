package fixture
import "os"
func visit(stop []bool) {
    chosen := "initial"
    for _, last := range stop {
        if last && len(os.Args) > 1 {
            chosen = os.Args[1]
            break
        }
        println(chosen)
        chosen = os.Getenv("EDITION")
    }
}
