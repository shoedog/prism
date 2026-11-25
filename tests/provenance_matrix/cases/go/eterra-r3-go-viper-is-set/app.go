package fixture
import "github.com/spf13/viper"
func run() {
  present := viper.IsSet("mode")
  mode := viper.GetString("mode")
  consume(present, mode)
}
func consume(...any) {}
