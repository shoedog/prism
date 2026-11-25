package main
import "example.test/viper"
func load() {
 mode := viper.GetString("mode")
 _ = mode
}
