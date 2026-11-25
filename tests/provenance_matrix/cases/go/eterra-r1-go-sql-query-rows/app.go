package main
import (
 "database/sql"
 "fmt"
)
func run() {
 db, _ := sql.Open("postgres", "dsn")
 rows, err := db.Query("select 1")
 fmt.Println(rows, err)
}
