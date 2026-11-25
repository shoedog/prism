package main
import "github.com/spf13/viper"
func run() {
 cfg := viper.New()
 mode := cfg.GetString("mode")
 println(mode)
}
