package fixture
import (
    "os"
    "github.com/spf13/viper"
)
func labels(file string) (string, string, error) {
    v := viper.New()
    v.SetConfigFile(file)
    if err := v.ReadInConfig(); err != nil { return "", "", err }
    if !v.IsSet("edition") { return "", "", nil }
    var book struct {
        Edition string `mapstructure:"edition"`
        Local string `mapstructure:"-"`
    }
    book.Local = os.Getenv("EDITION")
    if err := v.Unmarshal(&book); err != nil { return "", "", err }
    configured, local := book.Edition, book.Local
    return configured, local, nil
}
