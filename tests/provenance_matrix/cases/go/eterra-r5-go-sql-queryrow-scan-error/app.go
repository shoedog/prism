package fixture
import "database/sql"
func run() {
  db, _ := sql.Open("driver", "dsn")
  var name string
  err := db.QueryRow("select name from users").Scan(&name)
  consume(name, err)
}
func consume(...any) {}
