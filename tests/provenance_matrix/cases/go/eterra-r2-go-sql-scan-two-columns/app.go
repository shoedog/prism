package main
import "database/sql"
func run() {
 db, _ := sql.Open("driver", "dsn")
 row := db.QueryRow("select name, city from users")
 var name, city string
 row.Scan(&name, &city)
 println(name, city)
}
