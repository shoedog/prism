package fixture
import "database/sql"
func run(db *sql.DB) {
  pingErr := db.Ping()
  rows, _ := db.Query("select 1")
  consume(pingErr, rows)
}
func consume(...any) {}
