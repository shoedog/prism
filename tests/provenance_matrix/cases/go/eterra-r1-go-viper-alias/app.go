package main
import (
 "fmt"
 cfg "github.com/spf13/viper"
)
func run() {
 mode := cfg.GetString("mode")
 fmt.Println(mode)
}
