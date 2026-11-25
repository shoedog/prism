package main
import "github.com/spf13/viper"
func load() {
 values := viper.GetStringMap("service")
 present := viper.InConfig("service")
 _, _ = values, present
}
