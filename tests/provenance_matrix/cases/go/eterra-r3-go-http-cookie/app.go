package fixture
import "net/http"
func main() {
  http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
    cookie, _ := r.Cookie("session")
    println(cookie.Value)
  })
}
