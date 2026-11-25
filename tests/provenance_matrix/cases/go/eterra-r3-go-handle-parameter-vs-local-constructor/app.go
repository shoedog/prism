package fixture
import "database/sql"
func run(db *sql.DB) {
  rows, _ := db.Query("select 1")
  consume(rows)
}
func consume(...any) {}
