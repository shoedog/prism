package fixture
import conf "github.com/spf13/viper"
func run() {
  retries := conf.GetInt("retries")
  consume(retries)
}
func consume(...any) {}
