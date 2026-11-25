package main
import "github.com/spf13/viper"
func read() {
  delay := viper.GetDuration("delay")
  keys := viper.AllKeys()
  _, _ = delay, keys
}
