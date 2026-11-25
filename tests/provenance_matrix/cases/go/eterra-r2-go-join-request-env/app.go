package main
import ("net/http"; "os")
func handler(w http.ResponseWriter, r *http.Request) {
 value := os.Getenv("MODE")
 if r.Method == "POST" { value = r.FormValue("mode") }
 println(value)
}
