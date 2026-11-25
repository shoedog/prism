package fixture
import "github.com/spf13/viper"
func run() {
  v := viper.New()
  mode := v.GetString("mode")
  path := v.ConfigFileUsed()
  consume(mode, path)
}
func consume(...any) {}
