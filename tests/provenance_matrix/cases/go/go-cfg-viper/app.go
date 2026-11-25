package fixture

import "github.com/spf13/viper"

func read() string {
    mode := viper.GetString("mode")
    return mode
}
