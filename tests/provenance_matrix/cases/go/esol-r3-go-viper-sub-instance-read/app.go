package fixture

import settings "github.com/spf13/viper"

func port(path string) string {
  config := settings.New()
  config.SetConfigFile(path)
  if err := config.ReadInConfig(); err != nil { return "" }
  server := config.Sub("server")
  if server == nil { return "" }
  configuredPort := server.GetString("port")
  return configuredPort
}
