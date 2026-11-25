package main
import "database/sql"
func run() {
 db, _ := sql.Open("driver", "dsn")
 result, _ := db.Exec("delete from items")
 rows, _ := db.Query("select * from items")
 println(result, rows)
}
