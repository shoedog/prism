package fixture

import "github.com/spf13/viper"

func title() (string, bool) {
  captions := viper.GetStringMapString("captions")
  caption, present := captions["preview"]
  return caption, present
}
