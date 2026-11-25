package catalog
import (
    "net/http"
    "github.com/spf13/viper"
)
func register(mux *http.ServeMux) {
    mux.HandleFunc("/preview", func(out http.ResponseWriter, incoming *http.Request) {
        posted := viper.New()
        posted.SetConfigType("json")
        if err := posted.ReadConfig(incoming.Body); err != nil { return }
        saved := viper.New()
        saved.SetConfigFile("settings.json")
        if err := saved.ReadInConfig(); err != nil { return }
        preview := posted.GetString("edition")
        deployed := saved.GetString("edition")
        _, _ = preview, deployed
    })
}
