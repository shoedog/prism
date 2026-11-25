package fixture
import "github.com/spf13/viper"
func load() (*viper.Viper, string) {
    settings := viper.GetViper()
    mode := settings.GetString("ui.mode")
    return settings, mode
}
