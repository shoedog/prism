package main
import "database/sql"
func read() {
  db, _ := sql.Open("sqlite3", ":memory:")
  var name string
  var age int
  err := db.QueryRow("select name, age from users").Scan(&name, &age)
  _, _, _ = name, age, err
}
