package fixture
import (
    "os"
    "github.com/spf13/viper"
)
func labels(first bool) (string, string) {
    var left, right string
    if first {
        left = viper.GetString("edition")
        right = os.Getenv("LANG")
    } else {
        left = os.Getenv("LANG")
        right = viper.GetString("edition")
    }
    return left, right
}
