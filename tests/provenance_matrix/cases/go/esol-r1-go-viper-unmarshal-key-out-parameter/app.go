package fixture

import "github.com/spf13/viper"

func load() ([]string, error) {
    var labels []string
    decodeErr := viper.UnmarshalKey("ui.labels", &labels)
    return labels, decodeErr
}
