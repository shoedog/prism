package fixture

import "net/http"

type defaults struct{}
func (defaults) FormValue(string) string { return "fixed" }

func read(r *http.Request) string {
    if r != nil {
        r := defaults{}
        value := r.FormValue("q")
        return value
    }
    return ""
}

func fallback() string {
    r := defaults{}
    value := r.FormValue("mode")
    return value
}
