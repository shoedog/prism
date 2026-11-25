package main
import (
 "database/sql"
 "fmt"
)
func run() {
 db, _ := sql.Open("postgres", "dsn")
 closed := db.Close()
 fmt.Println(closed)
}
