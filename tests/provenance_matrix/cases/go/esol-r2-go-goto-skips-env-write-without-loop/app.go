package fixture
import "os"
func choose(label string) string {
    chosen := label
    goto done
    chosen = os.Getenv("MODE")
done:
    return chosen
}
