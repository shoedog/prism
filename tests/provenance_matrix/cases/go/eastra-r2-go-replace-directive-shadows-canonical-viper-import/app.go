package fixture
import settings "github.com/spf13/viper"
func edition() string {
    name := settings.GetString("edition")
    return name
}
